use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use tokio_postgres::Client;

/// Represents a pool of Postgres database connections.
///
/// The `PostgresPool` struct manages a pool of Postgres connections. It allows acquiring and releasing
/// connections, and ensures that the maximum number of connections is not exceeded.
#[derive(Clone)]
struct PostgresPool {
    connections: Arc<Mutex<Vec<Client>>>,
    max_connections: usize,
    database_url: String,
}


impl PostgresPool {
    /// Creates a new `PostgresPool` instance.
    ///
    /// This method creates a new `PostgresPool` instance with the specified `database_url` and `max_connections`.
    ///
    /// # Arguments
    ///
    /// * `database_url` - The URL of the Postgres database.
    /// * `max_connections` - The maximum number of connections allowed in the pool.
    ///
    /// # Returns
    ///
    /// A new `PostgresPool` instance.
    async fn new(database_url: &str, max_connections: usize) -> Self {
        let connections = Arc::new(Mutex::new(Vec::new()));

        for _ in 0..max_connections {
            let (client, connection) = tokio_postgres::connect(database_url, tokio_postgres::NoTls)
                .await
                .unwrap();

            // The connection object performs the actual communication with the database,
            // so spawn it off to run on its own.
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            connections.lock().unwrap().push(client);
        }

        PostgresPool {
            connections,
            max_connections,
            database_url: database_url.to_string(),
        }
    }

    /// Retrieves a connection from the pool.
    ///
    /// This method attempts to acquire a connection from the pool. If a connection is available, it is
    /// returned immediately. If the maximum number of connections
    /// has been reached, an error is returned.
    ///
    /// # Returns
    ///
    /// A `Result` containing the acquired `Client` connection if successful, or an error if the maximum
    /// number of connections has been reached.
    async fn get_connection(&self) -> Result<(usize, Client)> {
        let client = {
            let mut connections = self.connections.lock().unwrap();
            let index = connections.len() - 1;
            if let Some(conn) = connections.pop() {
                Some((index, conn))
            } else if connections.len() < self.max_connections {
                None
            } else {
                return Err(anyhow!("Max connections reached"));
            }
        };

        if let Some((index, client)) = client {
            return Ok((index, client));
        }

        Err(anyhow!("Max connections reached"))
    }

    /// Returns a connection to the pool.
    ///
    /// This method returns a connection to the pool, making it available for other code to use.
    ///
    /// # Arguments
    ///
    /// * `client` - The `Client` connection to return to the pool.
    fn return_connection(&self, client: Client) {
        let mut connections = self.connections.lock().unwrap();
        connections.push(client);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PostgresPool::new(
        "postgresql://postgres:supersecretpassword@localhost:5432/database",
        5,
    )
    .await;

    let mut tasks = Vec::new();

    for _ in 0..100 {
        let pool = pool.clone();

        tasks.push(tokio::spawn(async move {
            let (index, client) = pool.get_connection().await.unwrap();
            println!("Got connection on client: {:?}", index);
            pool.return_connection(client);
        }));
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}
