/// Logic related to database logistics; creating the database file, running migrations, etc.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use image::DynamicImage;
use ndarray::Array2;
use uuid::Uuid;

use crate::models::{NewBaseDirectory, NewFile, NewFileTag, NewImageFeaturesVitL14336Px, NewTag, NewFailedEncoding};
use crate::state::ConnectionPoolState;
use crate::{clip, db, preprocessing, schema};
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

pub fn get_connection_pool() -> anyhow::Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_path = get_db_path()?;

    let manager = ConnectionManager::<SqliteConnection>::new(db_path);

    Ok(Pool::builder()
        .test_on_check_out(true)
        .connection_customizer(Box::new(ConnectionOptions {
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(BUSY_TIMEOUT_SECONDS)),
        }))
        .build(manager)?)
}

pub fn init(pool_state: &tauri::State<'_, ConnectionPoolState>, populate_dummy_data: bool) -> anyhow::Result<()> {
    run_migrations(pool_state)?;
    
    // TODO Remove this eventually, it's just for testing. We will eventually be populating the DB via the UI and calling into more specific functions.
    if populate_dummy_data {
        populate_db_dummy_data_tags(pool_state)?;
        populate_image_features(pool_state)?;
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

fn get_db_path() -> anyhow::Result<String> {
    // TODO Pick a better spot for this, possibly in the app data directory?
    let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Unable to get home directory"))?;
    Ok(home_dir.to_str()
        .ok_or(anyhow::anyhow!("Unable to convert home directory PathBuf to string"))?
        .to_string() + "/.config/refrover/sqlite.refrover.db")
}

fn populate_db_dummy_data_tags(pool_state: &tauri::State<'_, ConnectionPoolState>) -> anyhow::Result<()>
{
    use crate::schema::{base_directories, file_tags, files, tags};

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

    let base_dir_id = Uuid::new_v4();
    let new_base_dir = NewBaseDirectory {
        id: &base_dir_id.to_string(),
        path: base_dir
    };

    // Insert the base directory
    diesel::insert_into(base_directories::table)
        .values(new_base_dir)
        .get_result::<(String, String)>(connection)?;
        
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

    let paths: anyhow::Result<Vec<PathBuf>> = fs::read_dir(base_dir)?
        .map(|entry| Ok(entry?.path()))
        .collect();
    let paths = paths?;

    let half_size = paths.len() / 2;

    // Insert the paths of half the files, and tag them as "Admins"
    for path in &paths[..half_size]
    {
        let relative_path = path.strip_prefix(base_dir)?.to_str().ok_or(anyhow::anyhow!("Error converting path to string"))?;

        let new_file_id = Uuid::new_v4();
        let new_file = NewFile {
            id: &new_file_id.to_string(),
            base_directory_id: &base_dir_id.to_string(),
            relative_path
        };

        diesel::insert_into(files::table)
            .values(new_file)
            .returning(files::id)
            .execute(connection)?;

        let new_file_tag = NewFileTag {
            file_id: &new_file_id.to_string(),
            tag_id: &admins_id.to_string(),
        };

        // This half gets the "Admins" tag
        diesel::insert_into(file_tags::table)
            .values(new_file_tag)
            .execute(connection)?;
    }

    // Do the same for the other half, for tag "Users"
    for path in &paths[half_size..]
    {
        let relative_path = path.strip_prefix(base_dir)?.to_str()
            .ok_or(anyhow::anyhow!("Error converting path to string"))?;

        let new_file_id = Uuid::new_v4();
        let new_file = NewFile {
            id: &new_file_id.to_string(),
            base_directory_id: &base_dir_id.to_string(),
            relative_path
        };

        diesel::insert_into(files::table)
            .values(new_file)
            .execute(connection)?;

        // This half gets the "Users" tag
        let new_file_tag = NewFileTag {
            file_id: &new_file_id.to_string(),
            tag_id: &users_id.to_string(),
        };

        diesel::insert_into(file_tags::table)
            .values(new_file_tag)
            .execute(connection)?;
    }
    
    Ok(())
}

// To be called after populate_dummy_data
// TODO This isn't a final function, I am hacking a test together,
//   but snippets of it will probably be useful.
//   For example, this totally hangs (could be IO, and we need to ensure we use a CUDA provider not CPU for our runtime)
//   But in any case, we'd want it to be some asynch task that's going and updating our KNN and table in the background
//   while users are allowing to continue using the app while that search information is updated for later.
//  TODO (Also check the system resources while we're doing this to see if we're hitting the GPU and IO stuff)
fn populate_image_features(pool_state: &tauri::State<'_, ConnectionPoolState>) -> anyhow::Result<()>
{
    // Query for all of the filepaths by joining the base_directories and files tables.

    use schema::image_features_vit_l_14_336_px::dsl::*;
    use schema::files::dsl::*;
    use schema::base_directories::dsl::*;
    use schema::failed_encodings;

    // Load our CLIP model.
    let clip = clip::Clip::new()?;

    let connection = &mut db::get_db_connection(pool_state)?;

    let all_files: Vec<(String, String, String)> = base_directories.inner_join(files)
        .select((schema::files::dsl::id, path, relative_path))
        .load::<(String, String, String)>(connection)?;

    for chunk in all_files.chunks(64)
    {
        // Get a vec of pathbufs to images
        let paths: anyhow::Result<Vec<(Uuid, PathBuf)>> = chunk.iter().map(|(uuid, base, rel) | {
            Ok((Uuid::parse_str(uuid)?, PathBuf::from(Path::new(&base).join(rel))))
        }).collect();
        let paths = paths?;
        
        // Load and preprocess our images.
        let images = preprocessing::load_image_batch(&paths);
        
        // Split images into those that succesfully loaded and those that failed.
        // Images may fail to load because they are not images, not found, etc.
        let (images, failed_images) = images.into_iter().partition::<Vec<_>, _>(|(_, img)| img.is_ok());

        // Handle images that succesfully loaded.
        // Unwrap the succesful images. This is safe because we just partitioned.
        let images: Vec<(Uuid, Box<DynamicImage>)> = images.into_iter().map(|(uuid, img)| (uuid, img.unwrap())).collect();
        let resized_images = preprocessing::resize_images(images);
        let image_clip_input = preprocessing::image_to_clip_format(resized_images);
        let image_encodings: Array2<f32> = clip.encode_image(image_clip_input)?;

        // Serialize each image encodings with bincode; convert the first axis of the ndarray to a vec
        let serialized_encodings: anyhow::Result<Vec<Vec<u8>>> = image_encodings.outer_iter().map(|row| {
            Ok(bincode::serialize(&row.to_vec())?)
        }).collect();
        let serialized_encodings = serialized_encodings?;
        
        // Insert the image encodings into the image_features_vit_l_14_336_px table
        // The encoding is serialized with serde.
        // The ID of the encoding is the same as the file ID.
        let new_image_features: Vec<NewImageFeaturesVitL14336Px> = chunk.iter().zip(serialized_encodings.iter()).map(|((file_id, _, _), encoding)| {
            NewImageFeaturesVitL14336Px {
                id: file_id,
                feature_vector: encoding
            }
        }).collect();
        
        diesel::insert_into(image_features_vit_l_14_336_px)
            .values(&new_image_features)
            .execute(connection)?;
        
        // Convert the failed images into NewFailedEncoding structs and insert them into the failed_encodings table.
        // The unwrap is safe because we just partitioned, so these are all Err results.
        let new_failed_encodings: Vec<NewFailedEncoding> = failed_images.into_iter().map(|(uuid, img)| {
            NewFailedEncoding {
                id: uuid.to_string(),
                error: img.as_ref().err().unwrap().to_string(),
                failed_at: None
            }
        }).collect();

        diesel::insert_into(failed_encodings::table)
            .values(&new_failed_encodings)
            .execute(connection)?;
    }

    Ok(())

    // The feature vectors should now be in the DB.
    // TODO - We'll actually have some load_knn() fn or similar that queries for that after this is done.
    //   We'll need a mechanism to keep that up-to-date with the DB.
}