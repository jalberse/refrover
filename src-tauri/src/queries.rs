/// Queries that operate on the database which contain core logic;
/// queries related to the database itself (e.g. to enable foreign keys)
/// are handled in the db module.

use std::path::PathBuf;

use diesel::dsl::{exists, select};
use diesel::sql_types::Text;
use diesel::prelude::*;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl, SqliteConnection};
use uuid::Uuid;

use crate::models::NewTagEdge;


pub fn add_tag_edge(start_vertex_id: Uuid, end_vertex_id: Uuid, source: &str, connection: &mut SqliteConnection)
{
   // See https://www.codeproject.com/Articles/22824/A-Model-to-Represent-Directed-Acyclic-Graphs-DAG-o
   use crate::schema::tag_edges;

   // TODO Wrap in transaction again

      let edge_exists = select(exists(tag_edges::table
         .filter(tag_edges::start_vertex_id.eq(start_vertex_id.to_string()))
         .filter(tag_edges::end_vertex_id.eq(end_vertex_id.to_string()))
         .filter(tag_edges::hops.eq(0)))).get_result(connection).expect("Error determining if edge exists");

      if edge_exists {
         // TODO Do nothing
         return;
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

      // TODO Cool, this ostensibly works. Let's build a more complex network (eg one from a figure in the article) and test that it works for e.g.
      //         inserting multiple UUIDs at a time, which I unfortunately don't think it will.

      // TODO Change our UUIDs to use some wrapper class
      //      https://github.com/diesel-rs/diesel/issues/364
      //      Would be binary (or I could go text) in the DB

      // TODO - yup, this is a problem. We can't insert multiple UUIDs at once.
      //    Realistically it's probably fine to do a loop and insert them one by one as needed.
      //    I think diesel probably has mechanisms available for some kind of for each thing to mix procedural and SQL stuff.
      //    Don't have performance brain about this, this client app is always going to be for one user on one machine and I doubt a small set of inserts will cause lag.
      // Hmm, something like this maybe? https://github.com/benwebber/sqlite3-uuid
      //    Could generate UUID within SQL...
      // TODO ************************************** Eh, for now just do the inserts in a loop, it's fine.
      //    Do a select statement (what's feeding values now), grab it, iterate rows and execute inserts. It's fine.

      // TODO Will the Uuid::new_v4() be a problem?
      //      For each inserted row is a new one generated, or is it the same for all inserted rows?
      //      The latter feels more likely, but would break uniqueness and the whole point of the id.
      //      If it's the former, then we're good.
      //      If it's trying to use the same ID, maybe we could create some auto-incrementing TMP table?
      //      And then use that to insert the values into the real table with new UUIDs?
      //   Hmm, if it is a problem, maybe I can go back to using an AUTOINCREMENT Integer field.
      //      If we do that, then all of this insertion etc is fine to do.
      //      UUIDs only become really important once we're sharing between users.
      //      And I suppose that at whatever point we interact between two DBs, we can have
      //         an intermediate layer that translates the IDs.
      //         e.g. we'd merge a DB in, which would in the process generate new UUIDs for all the IDs.

      // TODO May want to do select before the insert statement and use into_columns, as
      //    the insert_into() docs do. This ~works~ but might not be as clean?

      // Step 1: A's incoming edges to B
      diesel::insert_into(tag_edges::table)
         .values(
            tag_edges::table.select(
               (
                  Uuid::new_v4().to_string().into_sql::<Text>(), // new random UUID for the new edge
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
            )
         .execute(connection).expect("Ayudame");
      
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
      diesel::insert_into(tag_edges::table)
            .values(
               tag_edges::table.select(
                  (
                     Uuid::new_v4().to_string().into_sql::<Text>(), // new random UUID for the new edge
                     new_edge_id.to_string().into_sql::<Text>(), // EntryEdgeId
                     new_edge_id.to_string().into_sql::<Text>(), // DirectEdgeId
                     tag_edges::id, // ExitEdgeId
                     start_vertex_id.to_string().into_sql::<Text>(), // StartVertex
                     tag_edges::end_vertex_id, // EndVertex
                     tag_edges::hops + 1, // Hops
                     source.to_string().into_sql::<Text>() // Source
                  )
               )
               .filter(tag_edges::start_vertex_id.eq(end_vertex_id.to_string()))
            )
            .execute(connection).expect("Hither!");
      

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

      // Step 3: incoming edges of A to end vertex of B's outgoing edges
      // Since diesel does not support cross joins, use raw SQL.
      diesel::sql_query("INSERT INTO tag_edges (
            id,
            entry_edge_id,
            direct_edge_id,
            exit_edge_id,
            start_vertex_id,
            end_vertex_id,
            hops,
            source_id
         )
         SELECT
            ?
            , A.id
            , ?
            , B.id
            , A.start_vertex_id
            , B.end_vertex_id
            , A.hops + B.hops + 1
            , ?
         FROM tag_edges A
            CROSS JOIN tag_edges B
         WHERE A.end_vertex_id = ?
            AND B.start_vertex_id = ?")
         .bind::<Text, _>(Uuid::new_v4().to_string())
         .bind::<Text, _>(new_edge_id.to_string())
         .bind::<Text, _>(source)
         .bind::<Text, _>(start_vertex_id.to_string())
         .bind::<Text, _>(end_vertex_id.to_string())
         .execute(connection).expect("Thither!");
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