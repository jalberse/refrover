/// Logic related to database logistics; creating the database file, running migrations, etc.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use uuid::Uuid;

use crate::models::NewTag;
use crate::state::ConnectionPoolState;
use crate::db;
use crate::queries::{add_tag_edge, delete_tag_edge, get_edge_id};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

const BUSY_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug)]
pub struct ConnectionOptions {
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
    for ConnectionOptions
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

pub fn get_connection_pool(app_handle: &tauri::AppHandle) -> anyhow::Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_path = get_db_path(app_handle)?;

    let db_path_str = db_path.to_str().ok_or(anyhow::anyhow!("Error converting path to string"))?;

    // Ensure the db file exists at the path.
    // This doesn't run the migrations, we just ensure the file exists.
    if !Path::new(&db_path).exists() {
        SqliteConnection::establish(db_path_str)?;
    }

    let manager =
        ConnectionManager::<SqliteConnection>::new(db_path_str);

    let result = Pool::builder()
        .test_on_check_out(true)
        .connection_customizer(Box::new(ConnectionOptions {
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(BUSY_TIMEOUT_SECONDS)),
        }))
        .build(manager);

    match result {
        Ok(pool) => Ok(pool),
        Err(e) => Err(anyhow::anyhow!("Error creating connection pool: {:?}", e)),
    }
}

pub fn init(pool_state: &tauri::State<'_, ConnectionPoolState>, populate_dummy_data: bool) -> anyhow::Result<()> {
    run_migrations(pool_state)?;
    
    // TODO Remove this eventually, it's just for testing. We will eventually be populating the DB via the UI and calling into more specific functions.
    if populate_dummy_data {
        populate_db_dummy_data_tags(pool_state)?;
    }

    Ok(())
}

pub fn get_db_connection(pool_state: &tauri::State<'_, ConnectionPoolState>) -> anyhow::Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
    pool_state.get_connection()
}

fn run_migrations(pool_state: &tauri::State<'_, ConnectionPoolState>) -> anyhow::Result<()> {
    let mut connection = get_db_connection(pool_state)?;
    // Since this error size isn't known at compile-time, convert the error as necessary.
    let result = connection.run_pending_migrations(MIGRATIONS);
    if let Err(e) = result {
        return Err(anyhow::anyhow!("Error running migrations: {:?}", e));
    }
    anyhow::Ok(())
}

/// Gets the path to the SQLite database file.
/// Ensures that its parent directory exists.
/// The DB file may not exist yet; this function just gets the path.
/// Call diesel's `*Connection::establish()` to ensure the file exists.
fn get_db_path(app_handle: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app_handle.path_resolver().app_data_dir().ok_or(anyhow::anyhow!("Error getting app data path"))?;
    let path = dir.join("sqlite.refrover.db");
    fs::create_dir_all(&path.parent().ok_or(anyhow::anyhow!("Error getting parent directory"))?)?;
    Ok(path)
}

fn populate_db_dummy_data_tags(pool_state: &tauri::State<'_, ConnectionPoolState>) -> anyhow::Result<()>
{
    use crate::schema::{file_tags, files, tags};

    let base_dir = "D:\\refrover_photos";
    let connection = &mut db::get_db_connection(pool_state)?;

    // TODO - This would be initialized somewhere else. Probably populated when the db file is first created.
    let source_id = Uuid::new_v4();

    // Set up tags
    let admins_id = Uuid::new_v4();
    let admins_id_str = admins_id.to_string();
    let users_id = Uuid::new_v4();
    let users_id_str = users_id.to_string();
    let help_desk_id = Uuid::new_v4();
    let help_desk_id_str = help_desk_id.to_string();
    let ali_id = Uuid::new_v4();
    let ali_id_str = ali_id.to_string();
    let burcu_id = Uuid::new_v4();
    let burcu_id_str = burcu_id.to_string();
    let managers_id = Uuid::new_v4();
    let managers_id_str = managers_id.to_string();
    let technicians_id = Uuid::new_v4();
    let technicians_id_str = technicians_id.to_string();
    let can_id = Uuid::new_v4();
    let can_id_str = can_id.to_string();
    let demet_id = Uuid::new_v4();
    let demet_id_str = demet_id.to_string();
    let engin_id = Uuid::new_v4();
    let engin_id_str = engin_id.to_string();
    let fuat_id = Uuid::new_v4();
    let fuat_id_str = fuat_id.to_string();
    let gul_id = Uuid::new_v4();
    let gul_id_str = gul_id.to_string();
    let hakan_id = Uuid::new_v4();
    let hakan_id_str = hakan_id.to_string();
    let irmak_id = Uuid::new_v4();
    let irmak_id_str = irmak_id.to_string();
    let abctech_id = Uuid::new_v4();
    let abctech_id_str = abctech_id.to_string();
    let jale_id = Uuid::new_v4();
    let jale_id_str = jale_id.to_string();
    let new_tags = vec![
        NewTag { id: &admins_id_str, name: "admins" },
        NewTag { id: &users_id_str, name: "users" },
        NewTag { id: &help_desk_id_str, name: "HelpDesk" },
        NewTag { id: &ali_id_str, name: "Ali" },
        NewTag { id: &burcu_id_str, name: "Burcu" },
        NewTag { id: &managers_id_str, name: "Managers" },
        NewTag { id: &technicians_id_str, name: "Technicians" },
        NewTag { id: &can_id_str, name: "Can" },
        NewTag { id: &demet_id_str, name: "Demet" },
        NewTag { id: &engin_id_str, name: "Engin" },
        NewTag { id: &fuat_id_str, name: "Fuat" },
        NewTag { id: &gul_id_str, name: "Gul" },
        NewTag { id: &hakan_id_str, name: "Hakan" },
        NewTag { id: &irmak_id_str, name: "Irmak" },
        NewTag { id: &abctech_id_str, name: "ABC Tech" },
        NewTag { id: &jale_id_str, name: "Jale" },
    ];

    diesel::insert_into(tags::table)
        .values(&new_tags)
        .execute(connection)?;

    // https://www.codeproject.com/Articles/22824/A-Model-to-Represent-Directed-Acyclic-Graphs-DAG-o
    // Figure 5. Example of a DAG hierarchy.
    let _ = add_tag_edge(admins_id, help_desk_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(admins_id, ali_id, &source_id.to_string(), connection);

    let _ = add_tag_edge(users_id, ali_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(users_id, burcu_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(users_id, managers_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(users_id, technicians_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(users_id, can_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(users_id, engin_id, &source_id.to_string(), connection);

    let _ = add_tag_edge(help_desk_id, demet_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(help_desk_id, engin_id, &source_id.to_string(), connection);

    let _ = add_tag_edge(managers_id, fuat_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(managers_id, gul_id, &source_id.to_string(), connection);

    let _ = add_tag_edge(technicians_id, hakan_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(technicians_id, irmak_id, &source_id.to_string(), connection);
    let _ = add_tag_edge(technicians_id, abctech_id, &source_id.to_string(), connection);
    
    let _ = add_tag_edge(abctech_id, jale_id, &source_id.to_string(), connection);

    // Get the tag edge ID between technicians and abc tech.
    let tag_edge_id = get_edge_id(technicians_id, abctech_id, &source_id.to_string(), connection)?
        .ok_or(anyhow::anyhow!("Error getting tag edge ID"))?;

    // Delete the tag edge between technicians and abc tech.
    // This is just to show that we can delete edges.
    let _ = delete_tag_edge(tag_edge_id, connection);

    // let tree = get_tag_trees(connection);
    // println!("{:?}", tree);

    
    Ok(())
}
