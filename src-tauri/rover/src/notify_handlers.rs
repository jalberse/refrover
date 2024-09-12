
use std::path::PathBuf;

use log::{error, info, warn};
use notify_debouncer_full::{notify::{event::{CreateKind, ModifyKind, RemoveKind, RenameMode}, EventKind}, DebounceEventResult, DebouncedEvent};
use tauri::Manager;
use uuid::Uuid;

use crate::{ann, error::Error, interface::Payload, queries, state::{ClipState, ConnectionPoolState, SearchState}};

/// Called by the FsInnerWatcherState, which Tauri manages, to handle events.
pub struct FsEventHandler
{
    pub app_handle: tauri::AppHandle,
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

        // TODO Consider ignoring directory changes for now.
        //      The user *can* add them as watched directories through the UI.
        //      We can come back and add this later (and I do think we should, it's intuitive that we'd want to watch recursively).
        //      But for now, this is less important than *shipping something complete*.
        //      That does mean we want to detect directory changes though, and just log/ignore them.
        //      We don't want to treat a dir path as a file path.

        let event = &debounced_event.event;
        match event.kind {
            EventKind::Any => info!("EventKind::Any: {:?}", event),
            // Intentionally ignore Access events; nothing to do.
            EventKind::Access(_) => {},
            // TODO We should indeed handle create events, duh. If it's a directory, we should add it to the watcher, and do recursion ourselves on added files/dirs.
            //      We don't want recursive watching due to how Remove() will eg only remove the top level dir. We watch one level deep, and handle the recursive bits ourselves. 
            EventKind::Create(create_kind) => self.handle_create_event(debounced_event, create_kind, new_files)?,
            EventKind::Modify(modify_event_kind) => self.handle_modify_event(debounced_event, modify_event_kind, last_rename_from)?,
            // TODO Handle the remove event. For dirs, we won't worry about recursion - any internal dir should have its own watcher.
            //      Wait, would watchers have a race condition? Hmmm. That sucks lol. It's possible that we can expect the OS to emit the events in a bottom-up fasion,
            //      since would need to delete from the bottom up (unless some filesystem is leaving dangling dirs or something??)
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
                    // TODO Ignoring directories for now; they can be added through the UI.
                    //      Eventually, add the folder to the watcher, and add its contents to the DB, recursively.
                    info!("Create event for directory: {:?}", path);
                } else if path.is_file() {
                    new_files.push(path.clone());
                } else {
                    // Symlinks, etc - we'll ignore them for now.
                    warn!("Ignoring non-file/non-folder create event: {:?}", path);
                }
            }
            CreateKind::File => 
            {
                new_files.push(debounced_event.paths[0].clone());
            },
            CreateKind::Folder => {
                // TODO Add the folder to the watcher, and add its contents to the DB.
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
    ) -> anyhow::Result<()>
    {
        match modify_event_kind {
            ModifyKind::Any => {
                // Intentionally ignore Modify::Any. 
                // It triggers after a Create when copying files (at least on Windows).
                // If there is a scenario where we should handle it, we can determine that from logs.
                // info!("ModifyKind::Any: {:?}", debounced_event);
            },
            ModifyKind::Data(_) =>{
                // TODO Editing a photo in some programs e.g. Clip Studio Paint will actually result in a Remove/Create pair,
                //      rather than triggering a Modify::Data event. But in case some program/process *does* directly modify the data, we can similarly just handle it
                //      by treating it as we would a Remove/Create pair.
                todo!()
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
        let insert_result = queries::insert_files(&new_files, &mut connection);
        if let Err(e) = &insert_result {
            error!("Error inserting new files into DB: {:?}", e);
        }
        let new_files = insert_result.unwrap();

        {
            let clip_state = self.app_handle.state::<ClipState>();
            let clip = &clip_state.0.lock().unwrap().clip;
            clip.encode_image_files(&new_files, &mut connection)?;
        }

        {
            let file_ids = new_files.iter().map(|file| {
                file.0.clone()
            }).collect::<Vec<Uuid>>();
            
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
        let from_base_dir = from_path.parent().ok_or(anyhow::anyhow!("No parent for path: {:?}", from_path))?;
        let from_filename = from_path.file_name().ok_or(anyhow::anyhow!("No filename for path: {:?}", from_path))?;
        let to_base_dir = to_path.parent().ok_or(anyhow::anyhow!("No parent for path: {:?}", to_path))?;
        let to_filename = to_path.file_name().ok_or(anyhow::anyhow!("No filename for path: {:?}", to_path))?;

        // We expect the base dirs to match.
        if from_base_dir != to_base_dir {
            return Err(anyhow::anyhow!("Base dirs do not match for RenameMode::To event"));
        }

        let mut connection = self.app_handle.state::<ConnectionPoolState>().get_connection().expect("Unable to get connection from pool");

        let base_dir_id = queries::get_base_dir_id(
            from_base_dir.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;

        let base_dir_id = base_dir_id.ok_or(anyhow::anyhow!("Base dir ID not found"))?;

        let file_id = queries::get_file_id_from_base_dir_and_relative_path(
            &base_dir_id, 
            from_filename.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;

        let file_id = file_id.ok_or(anyhow::anyhow!("File ID not found"))?;

        queries::update_filename(
            &file_id, 
            to_filename.to_str().ok_or(Error::PathBufToString)?,
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
                    self.handle_remove_directory(path)?;
                } else if path.is_file() {
                    self.handle_remove_file(path)?;
                } else {
                    // Symlinks, etc - we'll ignore them for now.
                    warn!("Ignoring non-file/non-folder remove event: {:?}", path);
                }
            },
            RemoveKind::File => self.handle_remove_file(&debounced_event.paths[0])?,
            RemoveKind::Folder => self.handle_remove_directory(&debounced_event.paths[0])?,
        }

        Ok(())
    }

    fn handle_remove_file(
        &self,
        path: &PathBuf,
    ) -> anyhow::Result<()>
    {
        // TODO Note that sometimes, a Remove/Create pair will be used instead of a Modify event,
        // such as when editing a photo in Clip Studio Paint. In this case, we might not want
        // to remove the tags and other user-specified content, only things that are invalidated
        // (i.e. encodings and thumbnails). For now, I am going to simply remove everything,
        // and assume such cases don't normally come up (I could see people not wanting to lose
        // tags just because they e.g. did some redlining on reference photos, though).
        // I could imagine needing to change this in the future, but I think it's enough of an edge
        // case (I expect most users will mostly dump + forget files) that it's fine for now.

        let base_dir = path.parent().ok_or(anyhow::anyhow!("No parent for path: {:?}", path))?;
        let filename = path.file_name().ok_or(anyhow::anyhow!("No filename for path: {:?}", path))?;
        
        let mut connection = self.app_handle.state::<ConnectionPoolState>().get_connection().expect("Unable to get connection from pool");
        let base_dir_id = queries::get_base_dir_id(
            base_dir.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;

        let base_dir_id = base_dir_id.ok_or(anyhow::anyhow!("Base dir ID not found"))?;

        let file_id = queries::get_file_id_from_base_dir_and_relative_path(
            &base_dir_id, 
            filename.to_str().ok_or(Error::PathBufToString)?,
            &mut connection
        )?;

        let file_id = file_id.ok_or(anyhow::anyhow!("File ID not found"))?;

        queries::delete_file_tags(&file_id, &mut connection)?;
        queries::delete_failed_encoding(&file_id, &mut connection)?;
        queries::delete_encodings(&file_id, &mut connection)?;

        let thumbnail = queries::get_thumbnail_by_file_id(file_id, &mut connection)?;

        if let Some(thumbnail) = thumbnail {
            queries::delete_thumbnail_by_id(Uuid::parse_str(&thumbnail.id)?, &mut connection)?;

            // Delete the thumbnail from the filesystem.
            let app_data_path = self.app_handle.path_resolver().app_data_dir().ok_or(anyhow::anyhow!("Error getting app data path"))?;
            let full_path = app_data_path.join(&thumbnail.path);
            std::fs::remove_file(full_path)?;
        }

        Ok(())
    }

    fn handle_remove_directory(
        &self,
        path: &PathBuf,
    ) -> anyhow::Result<()>
    {
        // TODO We'll need to remove all the files in the contained folder from the DB.
        //      We can probabyl use handle_remove_file() for that for all the IDs contained.
        //      (or a shared fn since that needs to get IDs from the path, we get the IDs from the relational table).
        //      Then we need to remove the folder itself from the DB's base dir table.
        // TODO And then we need to remove the watcher for that folder.
        //      That won't be *this* watcher, but the one that's watching that folder.
        //      I'm vaguely worried about conflicts there?
        // TODO What about recursive? I'm kind of ~assuming~ that the OS will emit events in a bottom-up fashion.
        //      After all, it needs to delete the contents before the folder itself, itself.
        //      But if not, that's tricky.
        
        // TODO If the user adds a watched directory, and then it gets removed from the fs, what happens?
        //      Well, we could have a watcher that monitors each watched dir's parents, and checks for its removal.
        //      But if the dir is removed while the program is not running, we'd need to check for that anyways.
        //      Maybe that could be done on startup and then we have the parents watching for removal?
        //      Or we could just detect that it's missing when we try to access it and remove it then if we see it's missing?

        // TODO ... But for now, we're ignoring this. 
        info!("Remove event for directory: {:?}", path);
        Ok(())
    }
}