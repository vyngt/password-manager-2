use diesel::SqliteConnection;

pub trait ModelCRUD<TCreate>
where
    Self: Sized,
    TCreate: Sized,
{
    fn create(conn: &mut SqliteConnection, data: TCreate) -> Option<Self>;
    fn update(conn: &mut SqliteConnection, data: Self) -> Option<Self>;
    fn delete(conn: &mut SqliteConnection, _id: i64) -> bool;
    fn get(conn: &mut SqliteConnection, _id: i64) -> Option<Self>;
    fn all(conn: &mut SqliteConnection) -> Vec<Self>;
}
