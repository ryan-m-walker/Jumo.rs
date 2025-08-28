use tokio_postgres::NoTls;

pub struct PostgresMemory {
    client: Option<tokio_postgres::Client>,
}

impl PostgresMemory {
    pub fn new() -> Self {
        Self { client: None }
    }

    pub async fn init(&mut self) -> Result<(), anyhow::Error> {
        let (client, connection) =
            tokio_postgres::connect("host=localhost user=postgres", NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });

        self.client = Some(client);

        Ok(())
    }
}
