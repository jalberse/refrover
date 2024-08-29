
use std::path::PathBuf;

use diesel::SqliteConnection;
use log::{error, info, warn};
use notify_debouncer_full::{notify::{event::{ModifyKind, RenameMode}, EventKind}, DebounceEventResult, DebouncedEvent};
use tauri::Manager;

use crate::{error::Error, queries, state::ConnectionPoolState};

/// Called by the FsInnerWatcherState, which Tauri manages, to handle events.
pub struct FsEventHandler
{
    pub app_handle: tauri::AppHandle,
}

impl notify_debouncer_full::DebounceEventHandler for FsEventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        let connection_pool_state = self.app_handle.state::<ConnectionPoolState>();
        let mut connection = connection_pool_state.get_connection().expect("Unable to get connection from pool");

        match result {
            Ok(events) => {
                // Events are be stored in the following order:
                // 1. `remove` or `move out` event
                // 2. `rename` event.
                // 3. Other events

                // TODO Maybe an Option<RenameFrom> event that gets passed for each.
                //      It functions as a 1-sized-stack.
                //      Populate it with the RenameFrom when we encounter a FROM event.
                //      When we encounter a TO event, we "pop the stack" and use that to handle the event (ie know the old path).
                //      If there's a missing FROM event whenw we get a TO event, that's an error.
                //      If we're done processing events, that's an error.
                //      If we get two FROM events in a row, that's an error.
                //      We expect FROM/TO pairs follow one after the other in pairs. notify-debouncer-full should ensure this.
                //      TODO - I'm not 100% sure on this, because there are 
                // Thankfully other events can be handled individually, with notify-debouncer-full ensuring they're in a reasonable order

                let mut last_rename_from: Option<DebouncedEvent> = None;

                // TODO Note RenameMode::Both and RenameMode::Other are not used by notify-debouncer-full - check out the impl, they say so and intentionally ignore.
                //         (which I... kind of don't like? I assume they have a good reason though)
                //    eg rename "ANY" would have both paths according to docs:
                //    "The order of the paths is likely to be significant! For example, renames where both ends of
                //    the name change are known will have the "source" path first, and the "target" path last.""
                //    But the FROM/TO will have just the one path each, which we'll handle as explained above.
                info!("Handling events: {:?}", events);

                for event in events {
                    let result = Self::handle_event_inner(&event, &mut last_rename_from, &mut connection);
                    if let Err(e) = result {
                        // Log error and continue processing other events.
                        error!("Error handling event: {:?}", e);
                    }
                }
            },
            Err(e) => error!("Error handling event: {:?}", e),
        }
    }
}

impl FsEventHandler {
    fn handle_event_inner(
        debounced_event: &DebouncedEvent,
        last_rename_from: &mut Option<DebouncedEvent>,
        connection: &mut SqliteConnection
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
            EventKind::Create(_) => info!("EventKind::Create: {:?}", event),
            EventKind::Modify(modify_event_kind) => {
                Self::handle_modify_event(debounced_event, modify_event_kind, last_rename_from, connection)?;
            },
            // TODO Handle the remove event. For dirs, we won't worry about recursion - any internal dir should have its own watcher.
            //      Wait, would watchers have a race condition? Hmmm. That sucks lol. It's possible that we can expect the OS to emit the events in a bottom-up fasion,
            //      since would need to delete from the bottom up (unless some filesystem is leaving dangling dirs or something??)
            EventKind::Remove(_) => info!("EventKind::Remove: {:?}", event),
            // TODO Just like Any, just warn!() for now...
            EventKind::Other => info!("EventKind::Other: {:?}", event),
        }

        Ok(())
    }

    fn handle_modify_event(
        debounced_event: &DebouncedEvent,
        modify_event_kind: ModifyKind,
        last_rename_from: &mut Option<DebouncedEvent>,
        connection: &mut SqliteConnection,
    ) -> anyhow::Result<()>
    {
        match modify_event_kind {
            // Intentionally ignore Modify::Any
            // It triggers after a Create when copying files (at least on Windows), so we wouldn't want to re-do any work.
            // Log
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
                    RenameMode::Any => {
                        if debounced_event.paths.len() != 2 {
                            return Err(anyhow::anyhow!("RenameMode::Any event expected 2 paths, got {:?}", debounced_event.paths.len()));
                        }
                        let from_path = &debounced_event.paths[0];
                        let to_path = &debounced_event.paths[1];
                        Self::rename_file_in_db(from_path, to_path, connection)?;
                    },
                    RenameMode::To => {
                        if last_rename_from.is_none() {
                            error!("RenameMode::To event without a RenameMode::From event: {:?}", debounced_event);
                            return Err(anyhow::anyhow!("RenameMode::To event without a RenameMode::From event"));
                        }
                        let rename_from_event = last_rename_from.take().unwrap();

                        let from_path = &rename_from_event.paths[0];
                        let to_path = &debounced_event.paths[0];
                        Self::rename_file_in_db(from_path, to_path, connection)?;
                    },
                    RenameMode::From => {
                        if last_rename_from.is_some() {
                            error!("RenameMode::From event with a RenameMode::From event already present: {:?}", debounced_event);
                            return Err(anyhow::anyhow!("RenameMode::From event with a RenameMode::From event already present"));
                        }
                        *last_rename_from = Some(debounced_event.clone());
                        // We don't do any further processing here; we should process a matching RenameMode::To event next. 
                    },
                    RenameMode::Both => {
                        if debounced_event.paths.len() != 2 {
                            return Err(anyhow::anyhow!("RenameMode::Both event expected 2 paths, got {:?}", debounced_event.paths.len()));
                        }
                        let from_path = &debounced_event.paths[0];
                        let to_path = &debounced_event.paths[1];
                        Self::rename_file_in_db(from_path, to_path, connection)?;
                    }
                    RenameMode::Other => return Err(anyhow::anyhow!("Unexpected Event RenameMode::Other")),
                }
            },
            ModifyKind::Other => todo!(),
        }

        Ok(())
    }


    fn rename_file_in_db(
        from_path: &PathBuf,
        to_path: &PathBuf,
        connection: &mut SqliteConnection,
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

        let base_dir_id = queries::get_base_dir_id(
            from_base_dir.to_str().ok_or(Error::PathBufToString)?,
            connection
        )?;

        let base_dir_id = base_dir_id.ok_or(anyhow::anyhow!("Base dir ID not found"))?;

        let file_id = queries::get_file_id_from_base_dir_and_relative_path(
            &base_dir_id, 
            from_filename.to_str().ok_or(Error::PathBufToString)?,
            connection
        )?;

        let file_id = file_id.ok_or(anyhow::anyhow!("File ID not found"))?;

        queries::update_filename(
            &file_id, 
            to_filename.to_str().ok_or(Error::PathBufToString)?,
            connection
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