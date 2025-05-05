use tonic::transport::Channel;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::io::{self, AsyncBufReadExt};
use services::{
    payment_service_client::PaymentServiceClient,
    PaymentRequest,
    transaction_service_client::TransactionServiceClient,
    TransactionRequest,
    chat_service_client::ChatServiceClient,
    ChatMessage,
};

pub mod services {
    tonic::include_proto!("services");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Payment service
    let mut payment_client = PaymentServiceClient::connect("http://[::1]:50051").await?;
    let payment_request = tonic::Request::new(PaymentRequest {
        user_id: "user_123".to_string(),
        amount: 100.0,
    });
    let payment_response = payment_client.process_payment(payment_request).await?;
    println!("Payment Response: {:?}", payment_response.into_inner());

    // Transaction service
    let mut transaction_client = TransactionServiceClient::connect("http://[::1]:50051").await?;
    let transaction_request = tonic::Request::new(TransactionRequest {
        user_id: "user_123".to_string(),
    });
    let mut transaction_stream = transaction_client.get_transaction_history(transaction_request).await?.into_inner();
    while let Some(transaction) = transaction_stream.message().await? {
        println!("Transaction: {:?}", transaction);
    }

    // Chat service
    let channel = Channel::from_static("http://[::1]:50051").connect().await?;
    let mut chat_client = ChatServiceClient::new(channel);
    let (tx, rx): (Sender<ChatMessage>, Receiver<ChatMessage>) = mpsc::channel(32);

    // Spawn a task to read user input and send messages
    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let message = ChatMessage {
                user_id: "user_123".to_string(),
                message: line,
            };
            if tx.send(message).await.is_err() {
                eprintln!("Failed to send message to server");
                break;
            }
        }
    });

    // Send message stream to the server
    let chat_request = tonic::Request::new(ReceiverStream::new(rx));
    let mut response_stream = chat_client.chat(chat_request).await?.into_inner();

    while let Some(response) = response_stream.message().await? {
        println!("Server says: {:?}", response);
    }

    Ok(())
}
