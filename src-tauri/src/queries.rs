/// Queries that operate on the database which contain core logic;
/// queries related to the database itself (e.g. to enable foreign keys)
/// are handled in the db module.

use std::path::PathBuf;

use diesel::dsl::{exists, select};
use diesel::sql_types::Text;
use diesel::prelude::*;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl, SqliteConnection};
use uuid::Uuid;
use diesel::sql_types::Integer;

use crate::models::NewTagEdge;


pub fn add_tag_edge(start_vertex_id: Uuid, end_vertex_id: Uuid, source: &str, connection: &mut SqliteConnection) -> diesel::QueryResult<()>
{
   // See https://www.codeproject.com/Articles/22824/A-Model-to-Represent-Directed-Acyclic-Graphs-DAG-o
   use crate::schema::tag_edges;

      // TODO - Generating the UUIDs here necessitates a series of inserts rather than a batch insert from the select statements.
      //        Could there be a better way? Generating UUIDS in SQLite? Auto-incrementing IDs?
      //        This is likely totally fine, however, so I won't prematurely optimize.

      // TODO Wrap in transaction again

      // TODO Change our UUIDs to use some wrapper class
      //      https://github.com/diesel-rs/diesel/issues/364
      //      Would be binary (or I could go text) in the DB

      let edge_exists = select(exists(tag_edges::table
         .filter(tag_edges::start_vertex_id.eq(start_vertex_id.to_string()))
         .filter(tag_edges::end_vertex_id.eq(end_vertex_id.to_string()))
         .filter(tag_edges::hops.eq(0)))).get_result(connection).expect("Error determining if edge exists");

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
      let new_edge_id = Uuid::new_v4();

      let new_edge = NewTagEdge {
         id: &new_edge_id.to_string(),
         entry_edge_id: &new_edge_id.to_string(),
         direct_edge_id: &new_edge_id.to_string(),
         exit_edge_id: &new_edge_id.to_string(),
         start_vertex_id: &start_vertex_id.to_string(),
         end_vertex_id: &end_vertex_id.to_string(),
         hops: 0,
         source_id: source
      };

      diesel::insert_into(tag_edges::table)
         .values(new_edge)
         .execute(connection).expect("Error inserting new edge!!!");

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
            id: &Uuid::new_v4().to_string(),
            entry_edge_id: &entry_edge_id,
            direct_edge_id: &direct_edge_id,
            exit_edge_id: &exit_edge_id,
            start_vertex_id: &start_vertex_id,
            end_vertex_id: &end_vertex_id,
            hops: hops,
            source_id: &source
         };

         diesel::insert_into(tag_edges::table)
            .values(new_edge)
            .execute(connection).expect("Error inserting new edge!!!");
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
            id: &Uuid::new_v4().to_string(),
            entry_edge_id: &entry_edge_id,
            direct_edge_id: &direct_edge_id,
            exit_edge_id: &exit_edge_id,
            start_vertex_id: &start_vertex_id,
            end_vertex_id: &end_vertex_id,
            hops: hops,
            source_id: &source
         };

         diesel::insert_into(tag_edges::table)
            .values(new_edge)
            .execute(connection).expect("Error inserting new edge!!!");
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
         (SELECT
            A.id -- EntryEdgeId
            , B.id -- ExitEdgeId
            , A.start_vertex_id -- StartVertex
            , B.end_vertex_id -- EndVertex
            , A.hops + B.hops + 1 -- Hops
         FROM tag_edges A
            CROSS JOIN tag_edges B
         WHERE A.end_vertex_id = ?
            AND B.start_vertex_id = ?")
         .bind::<Text, _>(start_vertex_id.to_string())
         .bind::<Text, _>(end_vertex_id.to_string())
         .execute(connection).expect("Error creating temporary table");
      
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
          .load::<TempTagEdge>(connection)
          .expect("Error loading tmp edges");

      for tmp_edge in &tmp_edges {
         let new_edge = NewTagEdge {
            id: &Uuid::new_v4().to_string(), // Generate a new UUID for each row
            entry_edge_id: &tmp_edge.entry_edge_id,
            direct_edge_id: &new_edge_id.to_string(),
            exit_edge_id: &tmp_edge.exit_edge_id,
            start_vertex_id: &tmp_edge.start_vertex_id,
            end_vertex_id: &tmp_edge.end_vertex_id,
            hops: tmp_edge.hops,
            source_id: &source
         };

         diesel::insert_into(tag_edges::table)
            .values(new_edge)
            .execute(connection).expect("Error inserting new edge!!!");
      }

      Ok(())
}

// TODO Delete tag_edge function. Also complex. Other queries are simpler.

// Function to query for all files with a given tgag, including parent tags
pub fn query_files_with_tag(tag_id: Uuid, connection: &mut SqliteConnection) -> Vec<String>
{
    use crate::schema::{file_tags, files, base_directories};

    let tag_ids = find_containing_tags(tag_id, connection).into_iter().map(|tag_id| tag_id.to_string()).collect::<Vec<String>>();

    let file_ids = file_tags::table
        .filter(file_tags::tag_id.eq_any(tag_ids))
        .select(file_tags::file_id)
        .load::<String>(connection)
        .expect("Error loading file IDs");

    let files: Vec<(String, String)> = files::table
        .filter(files::id.eq_any(file_ids))
        .inner_join(base_directories::table.on(files::base_directory_id.eq(base_directories::id)))
        .select((base_directories::path, files::relative_path))
        .load::<(String, String)>(connection)
        .expect("Error loading files");

    let files = files.into_iter().map(|(parent_dir, relpath)| -> String {
        let parent_dir = PathBuf::from(parent_dir);
        let relpath = PathBuf::from(relpath);

        let file_path = parent_dir.join(relpath);

        file_path.to_string_lossy().into_owned()
    }).collect();

    files
}

// Returns tag_id and its parents
pub fn find_containing_tags(tag_id: Uuid, connection: &mut SqliteConnection) -> Vec<Uuid>
{
    // TODO this is simple now with DAG, do it.
    todo!()
}