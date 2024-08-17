
use log::{info, warn};
use notify;
use notify::event::{CreateKind, RemoveKind};

/// Called by the FsInnerWatcherState, which Tauri manages, to handle events.
/// During the .setup() we set up a FsEventHandler with the app handle, use it to create a watcher,
/// and then store that watcher in the FsWatcherState. This ensures that the watcher/handler are kept alive.
pub struct FsEventHandler
{
    pub app_handle: tauri::AppHandle,
}

impl notify::EventHandler for FsEventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        match event {
            Ok(event) => {
                Self::handle_event_inner(event);    
            }
            Err(e) => {
                println!("watch error: {:?}", e);
            }
        }
    }
}

impl FsEventHandler {
    fn handle_event_inner(event: notify::Event) {
        // Handle the various kinds of events that can occur.
        // Notice that we are chiefly concerned with the `Create` and `Remove` kinds,
        // and then creating/removing files/directories within those kinds.
        // Other cases may include things such as mounting/unmounting a filesystem,
        // which we will choose to not handle here for now. We may in the future! 
        // TODO Consider handling the Rename and DataModified kinds for EventKind::Modifty.
        //      Those might actually be nice to include, so users could edit photos or rename them
        //      and we could update them in the DB. We'd need to consider rename/data changes
        //      while the app *isn't* open, though, and I'm not sure we *could* do that.
        //      (how would we check against old data/names that we don't have access to?)
        match event.kind {
            notify::EventKind::Create(create_kind) => {
                match create_kind {
                    CreateKind::File => {
                        info!("New file created: {:?}", event.paths);
                        todo!();
                    }
                    CreateKind::Folder => {
                        info!("New directory created: {:?}", event.paths);
                        todo!();
                    }
                    CreateKind::Any => {
                        warn!("Unhandled create kind: {:?}", create_kind);
                    }
                    CreateKind::Other => {
                        warn!("Unhandled create kind: {:?}", create_kind);
                    },
                }
            }
            notify::EventKind::Any => { },
            notify::EventKind::Access(_) => { },
            notify::EventKind::Modify(_) => { },
            notify::EventKind::Remove(remove_kind) => {
                match remove_kind {
                    RemoveKind::File => {
                        info!("Event: File removed: {:?}", event.paths);
                        todo!();
                    },
                    RemoveKind::Folder => {
                        info!("Event: Folder removed: {:?}", event.paths);
                        todo!();
                    },
                    RemoveKind::Any => { warn!("Unhandled remove kind: {:?}", remove_kind); },
                    RemoveKind::Other => { warn!("Unhandled remove kind: {:?}", remove_kind); },
                }
            },
            notify::EventKind::Other => {},
        }
    }

    // TODO Create each of these functions, and just print out some details for now.
    //      Then do a bit of frontend work to test them out.
    //      Starting with, I suppose, the ability
    //      to add a watched directory, which will call a command that adds the watched dir to the watcher
    //      in the app state.
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