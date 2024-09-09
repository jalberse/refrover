
use std::path::PathBuf;

use log::{error, info};
use notify_debouncer_full::{notify::{event::{CreateKind, ModifyKind, RenameMode}, EventKind}, DebounceEventResult, DebouncedEvent};
use tauri::Manager;
use uuid::Uuid;

use crate::{ann, error::Error, models::{NewFailedEncoding, NewImageFeaturesVitL14336Px}, preprocessing, queries, state::{ClipState, ConnectionPoolState, SearchState}};

/// Called by the FsInnerWatcherState, which Tauri manages, to handle events.
pub struct FsEventHandler
{
    pub app_handle: tauri::AppHandle,
}

impl notify_debouncer_full::DebounceEventHandler for FsEventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        match result {
            Ok(events) => {
                // Events are be stored in the following order:
                // 1. `remove` or `move out` event
                // 2. `rename` event.
                // 3. Other events

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
            EventKind::Remove(_) => info!("EventKind::Remove: {:?}", event),
            // TODO Just like Any, just warn!() for now...
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
                // TODO Determine if it's a file or folder, and handle accordingly.
                let path = &debounced_event.paths[0];
                if path.is_dir() {
                    // TODO Add the folder to the watcher, and add its contents to the DB.
                    info!("Adding folder to watcher: {:?}", path);
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
            // Intentionally ignore Modify::Any. Note it triggers after a Create when copying files (at least on Windows).
            ModifyKind::Any => info!("ModifyKind::Any: {:?}", debounced_event),
            // TODO Editing a photo in some programs e.g. Clip Studio Paint will actually result in a Remove/Create pair,
            //      rather than triggering a Modify::Data event. But in case some program/process *does* directly modify the data, we can similarly just handle it
            //      by treating it as we would a Remove/Create pair.
            ModifyKind::Data(_) => todo!(),
            // We are ignoring Metadata events for now; log them for now so we can retroactively determine if there's something we should handle.
            ModifyKind::Metadata(event) => info!("ModifyKind::Metadata: {:?}", event),
            ModifyKind::Name(modify_name_kind) => {
                match modify_name_kind {
                    // TODO Potentially differentiate between directories and files here?
                    RenameMode::Any | RenameMode::Both => {
                        if debounced_event.paths.len() != 2 {
                            return Err(anyhow::anyhow!("{:?} event expected 2 paths, got {:?}", modify_event_kind, debounced_event.paths.len()));
                        }
                        let from_path = &debounced_event.paths[0];
                        let to_path = &debounced_event.paths[1];
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
                    RenameMode::Other => return Err(anyhow::anyhow!("Unexpected Event RenameMode::Other")),
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

        // TODO I think we also want to do this for new files created while the program isn't running,
        //      possibly in setup(). Handling all new_files set up (including finding them!).
        //      When we do so, we'll want to move a lot of this into shared functions.

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

    //      Then once we're actually watching some dirs, we can create/remove files and folders
    //      and check that we get those events as we'd expect.

    // TODO Functions for each of create/remove file/folder (and call from above)
    // Folders:
    // Add/remove contained files to/from DB, and add/remove folder to/from DB.
    // (ordering depending on if we are creating or removing, for FK constraints)
    // Also add/remove to the watcher itself (so probably pass app state in)
    //   We might need to store a reference to that in the NewImageFileHandler struct itself?
    // Basically anything with fileids we'd need to add/remove from the DB.
    // Including encodings.
    // Also set up fs allowances, eg: tauri::scope::FsScope::allow_directory(&app.fs_scope(), "D:\\refrover_photos", true)?;
    // Files:
    // Add/remove file to/from DB, and add/remove encodings to/from DB.
    //  Again we'd need appstate for the DB connection and CLIP.
    // 
    // The FsEventHandler has an app_handle that we can use to get the app state, db connection, etc.
}