/// Logic related to database logistics; creating the database file, running migrations, etc.

use std::fs;
use std::path::Path;
use std::str::FromStr;


use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde_json::de;
use uuid::Uuid;

use crate::models::{NewBaseDirectory, NewFile, NewFileTag, NewTag};
use crate::db;
use crate::queries::add_tag_edge;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn init() {
    if !db_file_exists() {
        create_db_file();
    }

    run_migrations();

    // TODO Remove this eventually, it's just for testing
    populate_db_dummy_data();
}

pub fn establish_db_connection() -> SqliteConnection {
    let db_path = get_db_path().clone();

    SqliteConnection::establish(db_path.as_str())
        .unwrap_or_else(|_| panic!("Error connecting to {}", db_path))
}

fn run_migrations() {
    let mut connection = establish_connection();
    connection.run_pending_migrations(MIGRATIONS).unwrap();
}

fn establish_connection() -> SqliteConnection {
    let db_path = "sqlite://".to_string() + get_db_path().as_str();

    SqliteConnection::establish(&db_path)
        .unwrap_or_else(|_| panic!("Error connecting to {}", db_path))
}

fn create_db_file() {
    let db_path = get_db_path();
    let db_dir = Path::new(&db_path).parent().unwrap();

    if !db_dir.exists() {
        fs::create_dir_all(db_dir).unwrap();
    }

    println!("Creating DB file at: {}", db_path);

    fs::File::create(db_path).unwrap();
}

fn db_file_exists() -> bool {
    let db_path = get_db_path();

    println!("Checking for DB file at: {}", db_path);

    Path::new(&db_path).exists()
}

fn get_db_path() -> String {
    // TODO I think that this should work fine - I was worried that diesel setup requires
    // knowing the location of the db, but I think once the  diesel CLI generates stuff in our source,
    // that won't matter and we don't need to update the env variable to the user's home. Check, though.
    let home_dir = dirs::home_dir().expect("Couldn't find home directory!");
    home_dir.to_str().unwrap().to_string() + "/.config/vizlib/sqlite.vizlib.db"
}

fn populate_db_dummy_data()
{
    use crate::schema::{base_directories, file_tags, files, tags};

    let base_dir = "D:\\vizlib_photos";
    let connection = &mut db::establish_db_connection();

    // TODO - This would be initialized somewhere else. Probably populated when the db file is first created.
    let source_id = Uuid::new_v4();

    // Set up tags
    let a_id = Uuid::new_v4();
    let a_id_str = a_id.to_string();
    let b_id = Uuid::new_v4();
    let b_id_str = b_id.to_string();
    let new_tags = vec![
        NewTag { id: &a_id_str, name: "a" },
        NewTag { id: &b_id_str, name: "b" },
    ];

    diesel::insert_into(tags::table)
        .values(&new_tags)
        .execute(connection)
        .expect("Error inserting tags");

    // TODO Test and build a larger network of tags, like the figure example from the article.
    //      Notably I am interested in unique IDs for batch inserts - I think we fail and want to test.
    //      A tmp table with autoincrementing IDs that is later inserted with unique IDs might work... idk.  

    add_tag_edge(a_id, b_id, &source_id.to_string(), connection);

    let base_dir_id = Uuid::new_v4();
    let new_base_dir = NewBaseDirectory {
        id: &base_dir_id.to_string(),
        path: base_dir
    };

    // Insert the base directory
    diesel::insert_into(base_directories::table)
        .values(new_base_dir)
        .get_result::<(String, String)>(connection)
        .expect("Error inserting base dir");
        
        
    // TODO When we actually let users choose a directory, we'll need to handle
    //    nesting. I think we'll do that by having a function like this that
    //    does just get the images in the directory, and that does NOT handle
    //    subdirectories. Instead, our UI will select all the subdirectories
    //    (handling that recursion for us) and then we just pass that set of
    //    directories all as their own base directories.
    // We also want to create an option to create a tag with the name of the base directory.
    //   So if you're importing an existing structure (like my current one) you can create the tags and relationships automatically.
    //    If it's a nested import, that involves adding the edges too.
    //    In UX they'd get to look at the tree of dirs and select which to import.

    let paths = fs::read_dir(base_dir).unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();

    let half_size = paths.len() / 2;

    // Insert the paths of half the files, and tag them as "A"
    for path in &paths[..half_size]
    {
        let relative_path = path.strip_prefix(base_dir).unwrap().to_str().unwrap();

        let new_file_id = Uuid::new_v4();
        let new_file = NewFile {
            id: &new_file_id.to_string(),
            base_directory_id: &base_dir_id.to_string(),
            relative_path
        };

        diesel::insert_into(files::table)
            .values(new_file)
            .returning(files::id)
            .execute(connection)
            .expect("Error inserting file");

        let new_file_tag = NewFileTag {
            file_id: &new_file_id.to_string(),
            tag_id: &a_id.to_string(),
        };

        // This half gets the "A" tag
        diesel::insert_into(file_tags::table)
            .values(new_file_tag)
            .execute(connection)
            .expect("error inserting file relationship with tag A");
    }

    // Do the same for the other half, for tag "B"
    for path in &paths[half_size..]
    {
        let relative_path = path.strip_prefix(base_dir).unwrap().to_str().unwrap();

        let new_file_id = Uuid::new_v4();
        let new_file = NewFile {
            id: &new_file_id.to_string(),
            base_directory_id: &base_dir_id.to_string(),
            relative_path
        };

        diesel::insert_into(files::table)
            .values(new_file)
            .execute(connection)
            .expect("Error inserting file");

        // This half gets the "B" tag
        let new_file_tag = NewFileTag {
            file_id: &new_file_id.to_string(),
            tag_id: &b_id.to_string(),
        };

        diesel::insert_into(file_tags::table)
            .values(new_file_tag)
            .execute(connection)
            .expect("error inserting file relationship with tag B");
    }
}