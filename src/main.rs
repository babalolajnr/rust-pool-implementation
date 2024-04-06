use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use tokio_postgres::{Client, Error};

/// Represents a pool of Postgres database connections.
struct PostgresPool {
    connections: Arc<Mutex<Vec<Client>>>,
    max_connections: usize,
    database_url: String,
}

/// Represents a pool of Postgres connections.
///
/// The `PostgresPool` struct manages a pool of Postgres connections. It allows acquiring and releasing
/// connections, and ensures that the maximum number of connections is not exceeded.
impl PostgresPool {
    /// Creates a new `PostgresPool` instance.
    ///
    /// # Arguments
    ///
    /// * `database_url` - The URL of the Postgres database.
    /// * `max_connections` - The maximum number of connections allowed in the pool.
    ///
    /// # Returns
    ///
    /// A new `PostgresPool` instance.
    fn new(database_url: &str, max_connections: usize) -> Self {
        let connections = Arc::new(Mutex::new(Vec::new()));

        PostgresPool {
            connections,
            max_connections,
            database_url: database_url.to_string(),
        }
    }

    /// Retrieves a connection from the pool.
    ///
    /// This method attempts to acquire a connection from the pool. If a connection is available, it is
    /// returned immediately. If no connection is available and the maximum number of connections has not
    /// been reached, a new connection is established and returned. If the maximum number of connections
    /// has been reached, an error is returned.
    ///
    /// # Returns
    ///
    /// A `Result` containing the acquired `Client` connection if successful, or an error if the maximum
    /// number of connections has been reached.
    async fn get_connection(&self) -> Result<Client> {
        let client = {
            let mut connections = self.connections.lock().unwrap();

            if let Some(conn) = connections.pop() {
                Some(conn)
            } else if connections.len() < self.max_connections {
                None
            } else {
                return Err(anyhow!("Max connections reached"));
            }
        };

        if let Some(client) = client {
            return Ok(client);
        }

        let (client, _) =
            tokio_postgres::connect(&self.database_url, tokio_postgres::NoTls).await?;

        Ok(client)
    }

    fn return_connection(&self, client: Client) {
        let mut connections = self.connections.lock().unwrap();
        connections.push(client);
    }
}

fn main() {
    println!("Hello, world!");
}
