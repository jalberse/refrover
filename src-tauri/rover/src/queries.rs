/// Queries that operate on the database which contain core logic;
/// queries related to the database itself (e.g. to enable foreign keys)
/// are handled in the db module.

use std::path::PathBuf;

use diesel::dsl::{exists, select};
use diesel::sql_types::Text;
use diesel::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, SqliteConnection};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;
use diesel::sql_types::Integer;

use crate::error::Error;
use crate::models::{ImageFeatureVitL14336Px, NewFile, NewTagEdge, NewThumbnail, RowsAffected, Thumbnail};
use crate::uuid::UUID;

pub fn add_tag_edge(start_vertex_id: UUID, end_vertex_id: UUID, source: UUID, connection: &mut SqliteConnection) -> diesel::QueryResult<()>
{
   // See https://www.codeproject.com/Articles/22824/A-Model-to-Represent-Directed-Acyclic-Graphs-DAG-o
   use crate::schema::tag_edges;

   let result = connection.transaction(|connection| {
      let edge_exists = select(exists(tag_edges::table
         .filter(tag_edges::start_vertex_id.eq(start_vertex_id.to_string()))
         .filter(tag_edges::end_vertex_id.eq(end_vertex_id.to_string()))
         .filter(tag_edges::hops.eq(0)))).get_result(connection)?;
   
      if edge_exists {
         // Do nothing
         return Ok(());
      }
   
      //    INSERT INTO Edge (
      //       StartVertex,
      //       EndVertex,
      //       Hops,
      //       Source)
      //    VALUES (
      //       @StartVertexId,
      //       @EndVertexId,
      //       0,
      //       @Source)
      //
      // SELECT @Id = SCOPE_IDENTITY()
      // UPDATE Edge
      //    SET EntryEdgeId = @Id
      //      , ExitEdgeId = @Id
      //      , DirectEdgeId = @Id 
      //    WHERE Id = @Id
   
      // The ID of the new direct edge. Other edges will be generated from this.
      let new_edge_id = Uuid::new_v4().into();
   
      let new_edge = NewTagEdge {
         id: new_edge_id,
         entry_edge_id: new_edge_id,
         direct_edge_id: new_edge_id,
         exit_edge_id: new_edge_id,
         start_vertex_id: start_vertex_id,
         end_vertex_id: end_vertex_id,
         hops: 0,
         source_id: source
      };
   
      diesel::insert_into(tag_edges::table)
         .values(new_edge)
         .execute(connection)?;
   
      //    -- step 1: A's incoming edges to B
      // INSERT INTO Edge (
      //       EntryEdgeId,
      //       DirectEdgeId,
      //       ExitEdgeId,
      //       StartVertex,
      //       EndVertex,
      //       Hops,
      //       Souce) 
      //    SELECT Id
      //       , @Id
      //       , @Id
      //       , StartVertex 
      //       , @EndVertexId
      //       , Hops + 1
      //       , @Source
      //    FROM Edge
      //    WHERE EndVertex = @StartVertexId
   
      let a_incoming_edges_to_b: Vec<(String, String, String, String, String, i32, String)> = tag_edges::table.select(
            (
            tag_edges::entry_edge_id,
            new_edge_id.to_string().into_sql::<Text>(), // the new edge ID
            new_edge_id.to_string().into_sql::<Text>(), // the new edge ID
            tag_edges::start_vertex_id,
            end_vertex_id.to_string().into_sql::<Text>(), // the end vertex ID
            tag_edges::hops + 1,
            source.to_string().into_sql::<Text>() // the source
            )
         )
         .filter(tag_edges::end_vertex_id.eq(start_vertex_id.to_string()))
         .load::<(String, String, String, String, String, i32, String)>(connection)?;
   
      // For each row in a_incoming_edges_to_b, insert it into the table, generating a unique UUID for each row.
      for (entry_edge_id, direct_edge_id, exit_edge_id, start_vertex_id, end_vertex_id, hops, source) in a_incoming_edges_to_b {
         let new_edge = NewTagEdge {
            id: Uuid::new_v4().into(),
            entry_edge_id: entry_edge_id.into(),
            direct_edge_id: direct_edge_id.into(),
            exit_edge_id: exit_edge_id.into(),
            start_vertex_id: start_vertex_id.into(),
            end_vertex_id: end_vertex_id.into(),
            hops: hops,
            source_id: source.into()
         };
   
         diesel::insert_into(tag_edges::table)
            .values(new_edge)
            .execute(connection)?;
      }
            
      // -- step 2: A to B's outgoing edges
      // INSERT INTO Edge (
      //    EntryEdgeId,
      //    DirectEdgeId,
      //    ExitEdgeId,
      //    StartVertex,
      //    EndVertex,
      //    Hops,
      //    Source) 
      // SELECT @Id
      //    , @Id
      //    , Id
      //    , @StartVertexId 
      //    , EndVertex
      //    , Hops + 1
      //    , @Source
      // FROM Edge
      // WHERE StartVertex = @EndVertexId
   
      // Step 2: A to B's outgoing edges
      let b_outgoing_edges: Vec<(String, String, String, String, String, i32, String)> = tag_edges::table.select(
            (
            new_edge_id.to_string().into_sql::<Text>(), // the new edge ID
            new_edge_id.to_string().into_sql::<Text>(), // the new edge ID
            tag_edges::id, 
            start_vertex_id.to_string().into_sql::<Text>(), // the start vertex ID
            tag_edges::end_vertex_id,
            tag_edges::hops + 1,
            source.to_string().into_sql::<Text>() // the source
            )
         )
         .filter(tag_edges::start_vertex_id.eq(end_vertex_id.to_string()))
         .load::<(String, String, String, String, String, i32, String)>(connection)?;
   
      for (entry_edge_id, direct_edge_id, exit_edge_id, start_vertex_id, end_vertex_id, hops, source) in b_outgoing_edges {
         let new_edge = NewTagEdge {
            id: Uuid::new_v4().into(),
            entry_edge_id: entry_edge_id.into(),
            direct_edge_id: direct_edge_id.into(),
            exit_edge_id: exit_edge_id.into(),
            start_vertex_id: start_vertex_id.into(),
            end_vertex_id: end_vertex_id.into(),
            hops: hops,
            source_id: source.into()
         };
   
         diesel::insert_into(tag_edges::table)
            .values(new_edge)
            .execute(connection)?;
      }
   
      // -- step 3: incoming edges of A to end vertex of B's outgoing edges
      // INSERT INTO Edge (
      //       EntryEdgeId,
      //       DirectEdgeId,
      //       ExitEdgeId,
      //       StartVertex,
      //       EndVertex,
      //       Hops,
      //       Source)
      //    SELECT A.Id
      //       , @Id
      //       , B.Id
      //       , A.StartVertex 
      //       , B.EndVertex
      //       , A.Hops + B.Hops + 1
      //       , @Source
      //    FROM Edge A
      //       CROSS JOIN Edge B
      //    WHERE A.EndVertex = @StartVertexId
      //      AND B.StartVertex = @EndVertexId
   
      // Diesel does not support cross joins, so we use raw SQL.
      // We create a temporary table to hold the results of the cross join, and then
      // iteratively insert the results into the tag_edges table, generating UUIDs.
      diesel::sql_query("
         CREATE TEMPORARY TABLE tmp_tag_edges AS
         SELECT
            A.id
            , B.id
            , A.start_vertex_id
            , B.end_vertex_id
            , A.hops + B.hops + 1
         FROM tag_edges A
            CROSS JOIN tag_edges B
         WHERE A.end_vertex_id = ?
            AND B.start_vertex_id = ?")
         .bind::<Text, _>(start_vertex_id.to_string())
         .bind::<Text, _>(end_vertex_id.to_string())
         .execute(connection)?;
         
      // Insert into the tag_edges table from the temporary table, generating UUIDs
      #[derive(QueryableByName)]
      #[table_name = "tmp_tag_edges"]
      struct TempTagEdge {
          #[sql_type = "Text"]
          entry_edge_id: String,
          #[sql_type = "Text"]
          exit_edge_id: String,
          #[sql_type = "Text"]
          start_vertex_id: String,
          #[sql_type = "Text"]
          end_vertex_id: String,
          #[sql_type = "Integer"]
          hops: i32,
      }
         
      let tmp_edges: Vec<TempTagEdge> = diesel::sql_query("SELECT * FROM tmp_tag_edges")
          .load::<TempTagEdge>(connection)?;
   
      for tmp_edge in &tmp_edges {
         let new_edge = NewTagEdge {
            id: Uuid::new_v4().into(), // Generate a new UUID for each row
            entry_edge_id: tmp_edge.entry_edge_id.clone().into(),
            direct_edge_id: new_edge_id.clone().into(),
            exit_edge_id: tmp_edge.exit_edge_id.clone().into(),
            start_vertex_id: tmp_edge.start_vertex_id.clone().into(),
            end_vertex_id: tmp_edge.end_vertex_id.clone().into(),
            hops: tmp_edge.hops,
            source_id: source.into()
         };
         
         diesel::insert_into(tag_edges::table)
            .values(new_edge)
            .execute(connection)?;
      }
   
      // Drop the tmp table
      diesel::sql_query("DROP TABLE tmp_tag_edges").execute(connection)?;

      Ok(())
   });

   result
}

/// Deletes the given tag edge from the database.
/// The edge must be a direct edge (hops = 0).
pub fn delete_tag_edge(id: UUID, connection: &mut SqliteConnection) -> diesel::QueryResult<()> {
   let result = connection.transaction(|connection| {
      diesel::sql_query("SELECT id FROM tag_edges WHERE id = ? AND hops = 0")
         .bind::<Text, _>(id.to_string())
         .execute(connection)?;
   
      // If the edge does not exist, return an error
      let rows_affected: RowsAffected = diesel::sql_query("SELECT changes() AS rows_affected").get_result(connection)?;
      let rows_affected = rows_affected.rows_affected;
      if rows_affected == 0 {
         return Err(diesel::result::Error::NotFound);
      }

      diesel::sql_query("CREATE TEMPORARY TABLE purge_list ( Id VARCHAR(36) PRIMARY KEY )")
         .execute(connection)?;

      // Step 1: Rows that were originally inserted with the first AddEdge call for this direct edge
      diesel::sql_query("INSERT INTO purge_list SELECT Id FROM tag_edges WHERE direct_edge_id = ?")
         .bind::<Text, _>(id.to_string())
         .execute(connection)?;

      // Step 2: scan and find all dependent rows that are inserted afterwards
      loop
      {
         diesel::sql_query("INSERT INTO purge_list
               SELECT id
               FROM tag_edges
               WHERE hops > 0
               AND ( entry_edge_id IN ( SELECT Id FROM purge_list )
               OR exit_edge_id IN ( SELECT Id FROM purge_list ))
               AND Id NOT IN ( SELECT Id FROM purge_list )")
            .execute(connection)?;
      
         // Get the nuber of rows effected by the last insert
         let rows_affected: RowsAffected = diesel::sql_query("SELECT changes() AS rows_affected").get_result(connection)?;
         let rows_affected = rows_affected.rows_affected;
         
         if rows_affected == 0 {
            break;
         }
      }

      // Delete the IDs in the purge list from the edges table
      diesel::sql_query("DELETE FROM tag_edges WHERE Id IN (SELECT Id FROM purge_list)")
      .execute(connection)?;

      // Drop the temporary table
      diesel::sql_query("DROP TABLE purge_list")
      .execute(connection)?;

      Ok(())
   });

   result
}

/// Get the direct edge ID given a start and end vertex ID and source ID.
/// If the edge does not exist, returns None, including if there is an indirect edge.
pub fn get_edge_id(start_vertex_id: UUID, end_vertex_id: UUID, source_id: UUID, connection: &mut SqliteConnection) -> anyhow::Result<Option<UUID>> {
   use crate::schema::tag_edges;

   let edge_id: Option<UUID> = tag_edges::table
      .select(tag_edges::id)
      .filter(tag_edges::start_vertex_id.eq(start_vertex_id.to_string()))
      .filter(tag_edges::end_vertex_id.eq(end_vertex_id.to_string()))
      .filter(tag_edges::source_id.eq(source_id))
      .filter(tag_edges::hops.eq(0))
      .first(connection)
      .optional()?;

   match edge_id
   {
    Some(edge_id) => Ok(Some(edge_id)),
    None => Ok(None),
   }
}

// TODO A function that gets the whole tree of tags.
// It should return JSON that can be used to render the tree.
// For example, in this format:
/**
const data = [
	{
		name: 'Parent',
		children: [{
			name: 'Child One'
		}, {
			name: 'Child Two'
		}, {
			name: 'Child Three',
			children: [{
				name: 'Grandchild One'
			}, {
				name: 'Grandchild Two'
			}]
		}]
	},
	{
		name: 'Child Three',
		children: [{
			name: 'Grandchild One'
		}, {
			name: 'Grandchild Two'
		}]
	},
	{
		name: 'Parent',
		children: [{
			name: 'Child One'
		}, {
			name: 'Child Two'
		}]
	}
];
*/

#[derive(Serialize, Deserialize)]
struct TagTreeNode 
{
   name: String,
   children: Option<Vec<TagTreeNode>>
}

pub fn get_tag_trees(connection: &mut SqliteConnection) -> anyhow::Result<String>
{
    use crate::schema::{tags, tag_edges};

   // Get all tags with no parents; i.e. all tag IDs which are not present in the end_vertex_id column of tag_edges.
   // These are the root nodes of the trees.
   let root_tag_ids: Vec<UUID> = tags::table
      .select(tags::id)
      .filter(tags::id.ne_all(tag_edges::table.select(tag_edges::end_vertex_id)))
      .load::<UUID>(connection)?;

   let mut trees = Vec::<TagTreeNode>::new();

   for root_tag_id in root_tag_ids {
      let root_tree = get_tag_tree(root_tag_id, connection)?;
      trees.push(TagTreeNode {
         name: get_tag_name(root_tag_id, connection)?,
         children: root_tree
      });
   }
   
   // Convert the trees to JSON with serde
   Ok(serde_json::to_string(&trees)?)
}

fn get_tag_tree(tag_id: UUID, connection: &mut SqliteConnection) -> anyhow::Result<Option<Vec<TagTreeNode>>>
{
   use crate::schema::tag_edges;

   // Get all children of the tag ID
   let children: Vec<UUID> = tag_edges::table
      .select(tag_edges::end_vertex_id)
      .filter(tag_edges::start_vertex_id.eq(tag_id))
      .filter(tag_edges::hops.eq(0))
      .load::<UUID>(connection)?;

   let mut out = Vec::<TagTreeNode>::new();
   for child in children {
      let child_name = get_tag_name(child, connection)?;
      let child_tree = get_tag_tree(child, connection)?;
      out.push(TagTreeNode {
         name: child_name,
         children: child_tree
      });
   }
   if out.is_empty() {
      return Ok(None);
   }
   Ok(Some(out))
}

pub fn get_tag_name(tag_id: UUID, connection: &mut SqliteConnection) -> anyhow::Result<String>
{
   use crate::schema::tags;

   let result = tags::table
      .select(tags::name)
      .filter(tags::id.eq(tag_id))
      .first(connection)?;

   Ok(result)
}

pub fn get_all_image_feature_data(connection: &mut SqliteConnection) -> anyhow::Result<Vec<ImageFeatureVitL14336Px>>
{
   use crate::schema::image_features_vit_l_14_336_px::dsl::*;

   let image_feature_data = image_features_vit_l_14_336_px
      .select(ImageFeatureVitL14336Px::as_select())
      .load(connection)?;

   Ok(image_feature_data)
}

pub fn get_image_feature_data(ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<Vec<ImageFeatureVitL14336Px>>
{
   use crate::schema::image_features_vit_l_14_336_px::dsl::*;

   let image_feature_data = image_features_vit_l_14_336_px
      .select(ImageFeatureVitL14336Px::as_select())
      .filter(id.eq_any(ids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>()))
      .load(connection)?;

   Ok(image_feature_data)
}

pub fn insert_thumbnail(thumbnail: &NewThumbnail, connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::thumbnails;

   diesel::insert_into(thumbnails::table)
      .values(thumbnail)
      .execute(connection)?;

   Ok(())
}

/// Gets the thumbnail data for the given file ID.
/// Returns NONE if the thumbnail does not exist in the table for the file ID.
pub fn get_thumbnail_by_file_id(file_id: UUID, connection: &mut SqliteConnection) -> anyhow::Result<Option<Thumbnail>>
{
   use crate::schema::thumbnails;

   let thumbnail = thumbnails::table
      .select(Thumbnail::as_select())
      .filter(thumbnails::file_id.eq(file_id))
      .first(connection)
      .optional()?;

   Ok(thumbnail)
}

pub fn delete_thumbnail_by_id(thumbnail_id: UUID, connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::thumbnails;

   diesel::delete(thumbnails::table.filter(thumbnails::id.eq(thumbnail_id)))
      .execute(connection)?;

   Ok(())
}

pub fn delete_thumbnails_by_file_ids(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::thumbnails;

   diesel::delete(thumbnails::table.filter(thumbnails::file_id.eq_any(file_ids)))
      .execute(connection)?;

   Ok(())
}

pub fn get_thumbnail_filepaths_by_file_ids(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<Vec<String>>
{
   use crate::schema::thumbnails;

   let paths: Vec<String> = thumbnails::table
      .select(thumbnails::path)
      .filter(thumbnails::file_id.eq_any(file_ids))
      .load(connection)?;

   Ok(paths)
}

pub fn get_filepaths(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<Vec<(UUID, PathBuf)>>
{
   use crate::schema::files;

   let ids_strings = file_ids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>();

   let filepaths: Vec<(UUID, String)> = files::table
      .select((files::id, files::filepath))
      .filter(files::id.eq_any(ids_strings))
      .load(connection)?;

   let out = filepaths.into_iter().map(|(id, filepath)| (id, PathBuf::from(filepath))).collect();

   Ok(out)
}

pub fn get_files_in_watched_directories(watched_dir_uuids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<Vec<UUID>>
{
   use crate::schema::files;

   let watched_dir_uuids = watched_dir_uuids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>();

   let file_ids: Vec<UUID> = files::table
      .select(files::id)
      .filter(files::watched_directory_id.eq_any(watched_dir_uuids))
      .load(connection)?;

   Ok(file_ids)
}

pub fn get_file_id_from_filepath(filepath: &str, connection: &mut SqliteConnection) -> anyhow::Result<Option<UUID>>
{
   use crate::schema::files;

   let file_id: Option<UUID> = files::table
      .select(files::id)
      .filter(files::filepath.eq(filepath))
      .first(connection)
      .optional()?;

   match file_id {
      Some(file_id) => Ok(Some(file_id)),
      None => Ok(None)
   }
}

pub fn update_filepath(file_id: &UUID, new_filepath: &str, connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::files;

   diesel::update(files::table.filter(files::id.eq(file_id)))
      .set(files::filepath.eq(new_filepath))
      .execute(connection)?;

   Ok(())
}

// Inserts the given files into the database, updating the base_directory and files tables.
// Returns the UUIDs of the inserted files.
pub fn insert_files(files: &[PathBuf], connection: &mut SqliteConnection, watched_dir_uuid: Option<UUID>) -> anyhow::Result<Vec<(UUID, PathBuf)>>
{
   use crate::schema::files;

   // TODO Consider using RETURNING clause instead to get the UUIDs/paths of inserted rows.
   let files = files.iter().map(|file| {
      let file_id = Uuid::new_v4().into();
      let filepath = file.to_str().map(|s| s.to_string()).ok_or(Error::PathBufToString)?;
      let new_file = NewFile {
         id: file_id,
         // TODO use paths-as-strings crate instead, possibly?
         filepath,
         watched_directory_id: watched_dir_uuid,
      };
      Ok((file_id, file.clone(), new_file))
   }).collect::<Vec<anyhow::Result<(UUID, PathBuf, NewFile)>>>();

   // Filter out any errors, logging them
   // TODO How should we handle this better?
   let files = files.into_iter().filter_map(|result| {
      match result {
         Ok(file) => Some(file),
         Err(e) => {
            log::error!("Error converting PathBuf to String. Path is likely not valid UTF-8: {:?}", e);
            None
         }
      }
   }).collect::<Vec<(UUID, PathBuf, NewFile)>>();

   let rows = files.iter().map(|(_, _, new_file)| new_file).collect::<Vec<&NewFile>>();

   diesel::insert_into(files::table)
      .values(rows)
      .execute(connection)?;

   Ok(files.into_iter().map(|(file_id, file, _)| (file_id.clone(), file.clone())).collect())
}

// In the case where we know the base directory ID, we can insert files directly.
pub fn insert_files_rows(files_rows: &[NewFile], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::files;

   diesel::insert_into(files::table)
      .values(files_rows)
      .execute(connection)?;

   Ok(())
}

/// Returns the UUID of the new watched directory.
pub fn insert_watched_directory(watched_directory: &str, connection: &mut SqliteConnection) -> anyhow::Result<UUID>
{
   use crate::schema::watched_directories;

   let watched_directory_uuid = Uuid::new_v4().into();
   let new_watched_directory = crate::models::NewWatchedDirectory {
      id: watched_directory_uuid,
      filepath: watched_directory,
   };

   diesel::insert_into(watched_directories::table)
      .values(new_watched_directory)
      .execute(connection)?;

   Ok(watched_directory_uuid)
}

pub fn watched_dir_exists(watched_directory: &str, connection: &mut SqliteConnection) -> anyhow::Result<bool>
{
   use crate::schema::watched_directories;

   let exists: bool = select(
      exists(
         watched_directories::table.filter(
            watched_directories::filepath.eq(watched_directory))))
      .get_result(connection)?;

   Ok(exists)
}

pub fn get_watched_directory_from_path(watched_directory: &str, connection: &mut SqliteConnection) -> anyhow::Result<Option<UUID>>
{
   use crate::schema::watched_directories;

   let uuid: Option<UUID> = watched_directories::table
      .select(watched_directories::id)
      .filter(watched_directories::filepath.eq(watched_directory))
      .first(connection)
      .optional()?;

   match uuid {
      Some(uuid) => Ok(Some(uuid)),
      None => Ok(None)
   }
}

fn delete_watched_directories(watched_directories_uuids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::watched_directories;

   // TODO This and in other fns, we don't need to convert to String anymore. UUIDs, yay.
   diesel::delete(watched_directories::table.filter(watched_directories::id.eq_any(watched_directories_uuids)))
      .execute(connection)?;

   Ok(())
}

fn delete_files(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::files;

   diesel::delete(files::table.filter(files::id.eq_any(file_ids)))
      .execute(connection)?;

   Ok(())
}

/// Deletes the given files from the database, cascading to resolve orphaned foreign keys
/// (in e.g. tag_edges, thumbnails, and encodings).
/// We don't use ON DELETE CASCADE to maintain greater control over the process, e.g.
/// to delete thumbnails from the disk during the deletion process as well (though you *could*
/// do so with ON DELETE CASCADE enabled). It's a choice to be more explicit in calling code.
/// (And we'd like to be consistent in this choice, which is important for e.g. tag_edges where
/// conflicting cascade paths may lead to unexpected behavior.)
pub fn delete_files_cascade(file_ids: &[UUID], connection: &mut SqliteConnection, app_handle: AppHandle) -> anyhow::Result<()>
{
   delete_files_tags(file_ids, connection)?;
   delete_failed_encodings(file_ids, connection)?;
   delete_files_encodings(file_ids, connection)?;

   let thumbnail_paths = get_thumbnail_filepaths_by_file_ids(file_ids, connection)?;
   // remove thumbnails from disk
   let app_data_path = app_handle.path_resolver().app_data_dir().ok_or(anyhow::anyhow!("Error getting app data path"))?;
   for path in thumbnail_paths {
      let thumbnail_path = app_data_path.join(path);
      std::fs::remove_file(thumbnail_path)?;
   }
   delete_thumbnails_by_file_ids(file_ids, connection)?;

   // All dependent tables should not reference the files anymore, so we can delete them.
   delete_files(file_ids, connection)?;

   Ok(())
}

pub fn delete_watched_directories_cascade(base_dir_ids: &[UUID], connection: &mut SqliteConnection, app_handle: AppHandle) -> anyhow::Result<()>
{
   // Note that since these IDs include those files in subdirectories, so we don't need to walk a tree.
   let file_ids = get_files_in_watched_directories(base_dir_ids, connection)?;
   delete_files_cascade(&file_ids, connection, app_handle)?;
   delete_watched_directories(base_dir_ids, connection)?;

   Ok(())
}

pub fn delete_files_encodings(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::image_features_vit_l_14_336_px;

   diesel::delete(image_features_vit_l_14_336_px::table.filter(image_features_vit_l_14_336_px::id.eq_any(file_ids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>())))
      .execute(connection)?;

   Ok(())
}

pub fn delete_failed_encodings(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::failed_encodings;

   diesel::delete(failed_encodings::table.filter(failed_encodings::id.eq_any(file_ids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>())))
      .execute(connection)?;

   Ok(())
}

pub fn delete_files_tags(file_ids: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
{
   use crate::schema::file_tags;

   diesel::delete(file_tags::table.filter(file_tags::file_id.eq_any(file_ids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>())))
      .execute(connection)?;

   Ok(())
}

#[cfg(test)]
mod tests
{
   use diesel::sqlite::Sqlite;
   use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
   
   use crate::{models::NewFile, schema::{watched_directories, files}};
   use super::*;

   // TODO As we are shipping this executable, we will want to actually embed migrations for the whole app,
   //   not just for the tests. See ROVER-111.
   pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
   
   fn establish_connection_in_memory() -> anyhow::Result<SqliteConnection>
   {
      let connection = SqliteConnection::establish(":memory:")?;
      Ok(connection)
   }

   fn run_migrations(connection: &mut impl MigrationHarness<Sqlite>) -> anyhow::Result<()> {
      // This will run the necessary migrations.
      //
      // See the documentation for `MigrationHarness` for
      // all available methods.
      let result  = connection.run_pending_migrations(MIGRATIONS);
      // The resulting error's size isn't known at compile time, so we make our own.
      // We don't need the MigrationVersions at the time of writing, so we ignore it.
      match result {
         Ok(_) => Ok(()),
         Err(e) => return Err(anyhow::anyhow!("Error running migrations: {:?}", e))
      }
   }

   fn setup() -> anyhow::Result<SqliteConnection>
   {
      let mut connection = establish_connection_in_memory()?;
      run_migrations(&mut connection)?;
      Ok(connection)
   }

   // TODO Test various queries.

}