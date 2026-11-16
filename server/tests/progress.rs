//! What the log says while the migrations run.

use rueder::progress;
use sqlx::PgPool;

/// The migrations run on an empty database as they would without anyone
/// watching them, and once more without anything to do.
#[sqlx::test(migrations = false)]
async fn migrates_an_empty_database(db: PgPool) {
    progress::migrate(&db).await.unwrap();
    progress::migrate(&db).await.unwrap();
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(applied as usize, sqlx::migrate!().iter().count());

    let how_far = progress::migrating(&db, 0).await.unwrap();
    assert!(
        how_far.starts_with("the last migration, the database has grown by "),
        "{how_far}"
    );
}
