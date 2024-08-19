
use log::{info, warn};
use notify_debouncer_full::{notify::EventKind, DebounceEventResult, DebouncedEvent};

/// Called by the FsInnerWatcherState, which Tauri manages, to handle events.
pub struct FsEventHandler
{
    pub app_handle: tauri::AppHandle,
}

impl notify_debouncer_full::DebounceEventHandler for FsEventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        match result {
            Ok(results) => {
                info!("Handling events: {:?}", results);

                for event in results {
                    Self::handle_event_inner(event);
                }
            },
            Err(e) => warn!("Error handling event: {:?}", e),
        }
    }
}

impl FsEventHandler {
    fn handle_event_inner(debounced_event: DebouncedEvent) {
        let event = debounced_event.event;
        match event.kind {
            // TODO We'll ignore EventKind::Any for now, since we don't have enough information. Do a warn!() in that case with the event data.
            //      We can collect that from logs to see if we need to handle. True for any unhandled case.
            EventKind::Any => info!("EventKind::Any: {:?}", event),
            // TODO We will intentionally ignore Access events
            EventKind::Access(_) => info!("EventKind::Access: {:?}", event),
            // TODO We should indeed handle create events, duh. If it's a directory, we should add it to the watcher, and do recursion ourselves on added files/dirs.
            //      We don't want recursive watching due to how Remove() will eg only remove the top level dir. We watch one level deep, and handle the recursive bits ourselves. 
            EventKind::Create(_) => info!("EventKind::Create: {:?}", event),
            // TODO Ignore Modify::Any and Modify::Metadata for now.
            //      Modify::Any triggers after a Create when copying files, so we wouldn't want to re-do any work.
            //      Do a remove/add cycle for Modify::Data. Some programs e.g. Clip Studio Paint will actually make a Remove/Create pair when overwriting data,
            //        rather than triggering a Modify::Data event. But in case some program/process *does* directly modify the data, we can similarly just handle it
            //        by treating it as we would a Remove/Create pair.
            // . Also handle Modify::Name.
            EventKind::Modify(_) => info!("EventKind::Modify: {:?}", event),
            // TODO Handle the remove event. For dirs, we won't worry about recursion - any internal dir should have its own watcher.
            //      Wait, would watchers have a race condition? Hmmm. That sucks lol.
            EventKind::Remove(_) => info!("EventKind::Remove: {:?}", event),
            // TODO Just like Any, just warn!() for now...
            EventKind::Other => info!("EventKind::Other: {:?}", event),
        }
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