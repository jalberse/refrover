
use log::{error, info, warn};
use notify_debouncer_full::{notify::{event::{ModifyKind, RenameMode}, EventKind}, DebounceEventResult, DebouncedEvent};

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

                let mut rename_from: Option<DebouncedEvent> = None;

                // TODO Note RenameMode::Both and RenameMode::Other are not used by notify-debouncer-full - check out the impl, they say so and intentionally ignore.
                //         (which I... kind of don't like? I assume they have a good reason though)
                //    eg rename "ANY" would have both paths according to docs:
                //    "The order of the paths is likely to be significant! For example, renames where both ends of
                //    the name change are known will have the "source" path first, and the "target" path last.""
                //    But the FROM/TO will have just the one path each, which we'll handle as explained above.
                info!("Handling events: {:?}", events);

                for event in events {
                    Self::handle_event_inner(&event, &mut rename_from);
                }
            },
            Err(e) => warn!("Error handling event: {:?}", e),
        }
    }
}

impl FsEventHandler {
    fn handle_event_inner(debounced_event: &DebouncedEvent, rename_from: &mut Option<DebouncedEvent>) {
        let event = &debounced_event.event;
        // TODO Don't get the basedir here necessarily (since different events could have multiple paths etc...) but we should have a fn that
        //      takes the path and gets the base directory for it.
        //      Or I guess we have that already, https://doc.rust-lang.org/stable/std/path/struct.PathBuf.html#method.parent filepath.parent() will get it (or None if non-applicable)
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
            //   rename FROM will just add to the Option<Event> "stack". If it's occupied, that's an error.
            //    rename TO will pop the stack and use the old path to handle the event. If there's no from, that's an error.
            EventKind::Modify(modify_event_kind) => {
                Self::handle_modify_event(debounced_event, modify_event_kind, rename_from)
            },
            // TODO Handle the remove event. For dirs, we won't worry about recursion - any internal dir should have its own watcher.
            //      Wait, would watchers have a race condition? Hmmm. That sucks lol.
            EventKind::Remove(_) => info!("EventKind::Remove: {:?}", event),
            // TODO Just like Any, just warn!() for now...
            EventKind::Other => info!("EventKind::Other: {:?}", event),
        }
    }

    fn handle_modify_event(debounced_event: &DebouncedEvent, modify_event_kind: ModifyKind, rename_from: &mut Option<DebouncedEvent>) 
    {
        match modify_event_kind {
            ModifyKind::Any => todo!(),
            ModifyKind::Data(_) => todo!(),
            ModifyKind::Metadata(_) => todo!(),
            ModifyKind::Name(modify_name_kind) => {
                // "Both" and "Other" should not be emitted by notify-debouncer-full, so we'll log an error and ignore such events.
                match modify_name_kind {
                    // TODO I think that with RenameMode::Any, we should be able to get the old and new paths in the one event together.
                    //      The documentation seems to suggest so. Code that assumption in, log an error msg if it's violated and ignore the event.
                    
                    RenameMode::Any => todo!(),
                    RenameMode::To => {
                        if rename_from.is_none() {
                            error!("RenameMode::To event without a RenameMode::From event: {:?}", debounced_event);
                            return;
                        }
                        let rename_from_event = rename_from.take().unwrap();
                        // TODO Use the rename_from event to get the "from" path, and the debounced_event to get the "to" path.
                        //   We can rename the file in the DB using that information - index on the name.                
                        //    The path is the absolute path, so we can grab that via some Rust code (like, "get the directory containing the thing at the end of the path")
                        //    We can check that the base dirs match. Note that the basedir stores the absolute path up to the dir, so we won't have overlap of like "oh same named folders in different places"
                        //  "take" handles the "pop" for us.
                        // Also, I think we really only need to update the name in the files table. The ID and encodings etc should be fine...
                    },
                    RenameMode::From => {
                        if rename_from.is_some() {
                            error!("RenameMode::From event with a RenameMode::From event already present: {:?}", debounced_event);
                            return;
                        }
                        *rename_from = Some(debounced_event.clone());
                        // We don't do any further processing here; we should process a matching RenameMode::To event next. 
                    },
                    RenameMode::Both => error!("Unexpected Event RenameMode::Both"),
                    RenameMode::Other => error!("Unexpected Event RenameMode::Other"),
                }
            },
            ModifyKind::Other => todo!(),
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