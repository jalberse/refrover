
use std::path::PathBuf;

use log::{error, info, trace};
use notify_debouncer_full::{notify::{event::{CreateKind, ModifyKind, RemoveKind, RenameMode}, EventKind}, DebounceEventResult, DebouncedEvent};
use tauri::Manager;
use uuid::Uuid;

use crate::{ann, error::Error, interface::Payload, queries, state::{ClipState, ConnectionPoolState, SearchState}};


pub const FS_WATCHER_DEBOUNCER_DURATION: std::time::Duration = std::time::Duration::from_millis(100);

/// Called by the FsInnerWatcherState, which Tauri manages, to handle events.
pub struct FsEventHandler
{
    pub app_handle: tauri::AppHandle,
    pub watch_directory_id: Uuid,
    pub watch_directory_path: PathBuf,
}

impl notify_debouncer_full::DebounceEventHandler for FsEventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {

        // TODO Consider file system ids:
        // https://docs.rs/file-id/0.2.1/file_id/index.html

        info!("Handling events...");
        let emit_result = self.app_handle.emit_all("fs-event", Payload { message: "analyzing...".to_string() });
        if emit_result.is_err()
        {
            error!("Error emitting fs-event: {:?}", emit_result);
        }

        // TODO On release ~200 images are taking *five seconds* to process?
        // Slower than I would think. I suppose we could investigate later.

        match result {
            Ok(events) => {
                // Events are be stored in the following order:
                // 1. `remove` or `move out` event
                // 2. `rename` event.
                // 3. Other events

                info!("Event count: {:?}", events.len());

                // Acts as a one-element-stack for the last `rename from` event,
                // so that the following `rename to` event can be processed.
                let mut last_rename_from: Option<DebouncedEvent> = None;
                // We cache new files rather than handling them immediately so that
                // we can process them in batches.
                let mut new_files: Vec<PathBuf> = Vec::new();
                for debounced_event in events {
                    let result = self.handle_event_inner(
                        &debounced_event,
                        &mut last_rename_from,
                        &mut new_files);
                    if let Err(e) = result {
                        // Log error and continue processing other events.
                        error!("Error handling event: {:?}", e);
                    }
                }

                if last_rename_from.is_some() {
                    error!("Last RenameMode::From event without a RenameMode::To event: {:?}", last_rename_from);
                }

                let result = self.handle_new_files(&new_files);
                if let Err(e) = result {
                    // Log error and continue processing other events.
                    error!("Error handling new files: {:?}", e);
                }
            },
            Err(e) => error!("Error handling event: {:?}", e),
        }

        info!("Done handling events.");
        let emit_result = self.app_handle.emit_all("fs-event", Payload { message: "rover-analyzer".to_string() });
        if emit_result.is_err()
        {
            error!("Error emitting fs-event: {:?}", emit_result);
        }
    }
}

impl FsEventHandler {
    fn handle_event_inner(
        &self,
        debounced_event: &DebouncedEvent,
        last_rename_from: &mut Option<DebouncedEvent>,
        new_files: &mut Vec<PathBuf>,
    ) -> anyhow::Result<()> {
        // Handle various events.
        // If there's a class of events that we ignore, we may log the event so we can retroactively determine if it's something we should be handling.
        // Event Kinds are not always intuitive; for example, editing a file in some programs will result in a Remove/Create sequence, rather than a Modify::Data event,
        // and ModifyKind::Any will trigger after a Create event when copying files into the watched directory (at least on Windows).
        // Further, Any/Other event fallbacks are inherently vague, and filesystem events are platform-dependent.
        // Extensive logging is therefore useful for refining our event handling for various scenarios.

        // For now, we ignore directories (and symlinked files, etc).
        // TODO - Consider recursively watching for files. If the watcher is recursive, then we'll catch new
        //        files added to subdirectories (and new subdirectories, I think).
        //        But, those files wouldn't have a basedir entry in the DB necessarily.
        //        For now, we're just watching flat directories for the MVP.
        //        Recursively watching makes a lot of sense (users may dump new folders in, we want to watch
        //        the contained files).
        //        Maybe in the API we check if newly added watched directories are already watched
        //        (ie they have a parent recursively watching them) and we can say "we can't add that directory,
        //        it's already being watched" as a dialog or something.
        //        That way we get recursive watching, and the user is expected to just add top-level directories
        //        that they want to watch.
        //        The complication there is that we *would* need to handle recursive base directories.
        //        The watched/basedir tables then don't capture the same concept.
        //        TODO - Then I'm tempted to simply do away with the basedir table and store the absolute path of every file.
        //        We store redundant data (the path to the base dir) but it could possibly simplify things...
        // TODO If we recursively watch and remove a directory, will the OS emit a remove for all the contained files?
        //        Or just the directory? If the latter, we'd need to handle getting all the contained files (and recursive dirs?)
        //        that are getting deleted.

        let event = &debounced_event.event;
        match event.kind {
            EventKind::Any => info!("EventKind::Any: {:?}", event),
            EventKind::Access(_) => {},
            EventKind::Create(create_kind) => self.handle_create_event(debounced_event, create_kind, new_files)?,
            EventKind::Modify(modify_event_kind) => self.handle_modify_event(debounced_event, modify_event_kind, last_rename_from, new_files)?,
            EventKind::Remove(remove_kind) => {
                info!("EventKind::Remove: {:?}", event);
                info!("RemoveKind: {:?}", remove_kind);
                self.handle_remove_event(debounced_event, remove_kind)?;
            },
            EventKind::Other => info!("EventKind::Other: {:?}", event),
        }

        Ok(())
    }

    fn handle_create_event(
        &self,
        debounced_event: &DebouncedEvent,
        create_kind: CreateKind,
        new_files: &mut Vec<PathBuf>,
    ) -> anyhow::Result<()>
    {
        match create_kind
        {
            CreateKind::Any | CreateKind::Other => {
                let path = &debounced_event.paths[0];
                if path.is_dir() {
                    info!("Create event for directory: {:?}", path);
                } else if path.is_file() {
                    new_files.push(path.clone());
                } else {
                    // Symlinks, etc - we'll ignore them for now.
                    info!("Ignoring non-file/non-folder create event: {:?}", path);
                }
            }
            CreateKind::File => 
            {
                new_files.push(debounced_event.paths[0].clone());
            },
            CreateKind::Folder => {
                info!("CreateKind::Folder event for: {:?}", debounced_event.paths[0]);
            },
        }

        Ok(())
    }

    fn handle_modify_event(
        &self,
        debounced_event: &DebouncedEvent,
        modify_event_kind: ModifyKind,
        last_rename_from: &mut Option<DebouncedEvent>,
        new_files: &mut Vec<PathBuf>,
    ) -> anyhow::Result<()>
    {
        match modify_event_kind {
            ModifyKind::Any => {
                // Intentionally ignore Modify::Any. 
                // It triggers after a Create when copying files (at least on Windows).
                // If there is a scenario where we should handle it, we can determine that from logs.
                trace!("ModifyKind::Any: {:?}", debounced_event);
            },
            ModifyKind::Data(_) =>{
                let path = &debounced_event.paths[0];
                if path.is_dir() {
                    info!("Modify event for directory: {:?}", path);
                } else if path.is_file() {
                    // If the data has changed, then the encodings and thumbnail are no longer valid.
                    // We'll handle it as if the file was removed and re-added, which is actually what is
                    // emitted if e.g. a file is edited in Clip Studio Paint.
                    // TODO Possibly handle this better. My concern is this will also remove tags.
                    // But for now, this should be "OK".
                    self.handle_remove_file(path)?;
                    new_files.push(path.clone());
                } else {
                    // Symlinks, etc - we'll ignore them for now.
                    trace!("Ignoring non-file/non-folder modify event: {:?}", path);
                }
            },
            ModifyKind::Metadata(event) => {
                // We are ignoring Metadata events for now;
                // log them for now so we can retroactively determine if there's something we should handle.
                info!("ModifyKind::Metadata: {:?}", event);
            },
            ModifyKind::Name(modify_name_kind) => {
                match modify_name_kind {
                    RenameMode::Any | RenameMode::Both | RenameMode::Other => {
                        if debounced_event.paths.len() != 2 {
                            return Err(anyhow::anyhow!("{:?} event expected 2 paths, got {:?}", modify_event_kind, debounced_event.paths.len()));
                        }
                        let from_path = &debounced_event.paths[0];
                        let to_path = &debounced_event.paths[1];

                        if from_path.is_dir() && to_path.is_dir()
                        {
                            info!("Intentionally ignoring RenameMode::Both event for directories: {:?} -> {:?}", from_path, to_path);
                            return Ok(());
                        } else if from_path.is_dir() || to_path.is_dir() {
                            return Err(anyhow::anyhow!("File/directory mismatch in RenameMode::Both event"));
                        }

                        self.rename_file_in_db(from_path, to_path)?;
                    },
                    RenameMode::To => {
                        if last_rename_from.is_none() {
                            error!("RenameMode::To event without a RenameMode::From event: {:?}", debounced_event);
                            return Err(anyhow::anyhow!("RenameMode::To event without a RenameMode::From event"));
                        }
                        let rename_from_event = last_rename_from.take().unwrap();

                        let from_path = &rename_from_event.paths[0];
                        let to_path = &debounced_event.paths[0];

                        if from_path.is_dir() && to_path.is_dir()
                        {
                            info!("Intentionally ignoring RenameMode::Both event for directories: {:?} -> {:?}", from_path, to_path);
                            return Ok(());
                        } else if from_path.is_dir() || to_path.is_dir() {
                            return Err(anyhow::anyhow!("File/directory mismatch in RenameMode::Both event"));
                        }

                        self.rename_file_in_db(from_path, to_path)?;
                    },
                    RenameMode::From => {
                        if last_rename_from.is_some() {
                            error!("RenameMode::From event with a RenameMode::From event already present: {:?}", debounced_event);
                            return Err(anyhow::anyhow!("RenameMode::From event with a RenameMode::From event already present"));
                        }
                        *last_rename_from = Some(debounced_event.clone());
                        // We don't do any further processing here; we should process a matching RenameMode::To event next. 
                    },
                }
            },
            ModifyKind::Other => todo!(),
        }

        Ok(())
    }


    fn handle_new_files(
        &self,
        new_files: &[PathBuf],
    ) -> anyhow::Result<()>
    {
        // Insert the files into the DB and get their new UUIDs.
        let mut connection = self.app_handle.state::<ConnectionPoolState>().get_connection().expect("Unable to get connection from pool");
        let insert_result = queries::insert_files(&new_files, &mut connection, &Some(self.watch_directory_id));
        if let Err(e) = &insert_result {
            error!("Error inserting new files into DB: {:?}", e);
        }
        let new_files = insert_result.unwrap();

        let file_ids = new_files.iter().map(|file| {
            file.0.clone()
        }).collect::<Vec<Uuid>>();
        
        {
            let clip_state = self.app_handle.state::<ClipState>();
            let clip = &clip_state.0.lock().unwrap().clip;
            clip.encode_image_files(&file_ids, &mut connection)?;
        }

        {
            let mut connection = self.app_handle.state::<ConnectionPoolState>().get_connection().expect("Unable to get connection from pool");
            
            let image_features = queries::get_image_feature_data(&file_ids, &mut connection)?;
            
            let hnsw_elements = ann::convert_rows_to_hnsw_elements(&image_features)?;
            
            let search_state = self.app_handle.state::<SearchState>();
            let mut search_inner = search_state.0.lock().unwrap();
            let hnsw = &mut search_inner.hnsw;
            hnsw.insert_slice(hnsw_elements);
        }

        // TODO Lower priority: Possibly generate thumbnails here.
        //      Low priority since generating them as-needed is fine for now.
        //      Could be done async with encodings.

        Ok(())
    }

    fn rename_file_in_db(
        &self,
        from_path: &PathBuf,
        to_path: &PathBuf,
    ) -> anyhow::Result<()>
    {
        // TODO Consider using OS file system ids here - they should match, I think.

        let mut connection = self.app_handle.state::<ConnectionPoolState>().get_connection().expect("Unable to get connection from pool");

        let file_id = queries::get_file_id_from_filepath(
            from_path.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;

        if file_id.is_none() {
            error!("File ID not found for path: {:?}", from_path);
            return Err(anyhow::anyhow!("File ID not found for path: {:?}", from_path));
        }
        let file_id = file_id.unwrap();

        queries::update_filepath(
            &file_id, 
            to_path.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;

        Ok(())
    }


    fn handle_remove_event(
        &self,
        debounced_event: &DebouncedEvent,
        remove_kind: RemoveKind,
    ) -> anyhow::Result<()>
    {
        match remove_kind
        {
            RemoveKind::Any | RemoveKind::Other => {
                let path = &debounced_event.paths[0];
                if path.is_dir() {
                    info!("Remove event for directory: {:?}", path);
                } else if path.is_file() {
                    self.handle_remove_file(path)?;
                } else {
                    // Symlinks, etc - we'll ignore them for now.
                    trace!("Ignoring non-file/non-folder remove event: {:?}", path);
                }
            },
            RemoveKind::File => self.handle_remove_file(&debounced_event.paths[0])?,
            RemoveKind::Folder => {
                info!("RemoveKind::Folder event for: {:?}", debounced_event.paths[0]);
            },
        }

        Ok(())
    }

    fn handle_remove_file(
        &self,
        path: &PathBuf,
    ) -> anyhow::Result<()>
    {
        // Note that sometimes, a Remove/Create pair will be used instead of a Modify event,
        // such as when editing a photo in Clip Studio Paint. In this case, we might not want
        // to remove the tags and other user-specified content, only things that are invalidated
        // (i.e. encodings and thumbnails). For now, I am going to simply remove everything,
        // and assume such cases don't normally come up (I could see people not wanting to lose
        // tags just because they e.g. did some redlining on reference photos, though).
        // I could imagine needing to change this in the future, but I think it's enough of an edge
        // case (I expect most users will mostly dump + forget files) that it's fine for now.

        let mut connection = self.app_handle.state::<ConnectionPoolState>().get_connection().expect("Unable to get connection from pool");
        
        // Get the file ID from the DB by quering the filepath
        let file_id = queries::get_file_id_from_filepath(
            path.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;
        let file_id = file_id.ok_or(anyhow::anyhow!("File ID not found"))?;
        
        queries::delete_files_cascade(&[file_id], &mut connection, self.app_handle.clone())?;

        Ok(())
    }
}