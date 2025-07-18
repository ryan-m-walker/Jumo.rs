// let (ws_tx, mut ws_rx) = mpsc::channel(100);
// let elevenlabs_api_key_clone = elevenlabs_api_key.clone();
//
// tokio::spawn(async move {
//     let mut request =
//         "wss://api.elevenlabs.io/v1/text-to-speech/GfVVOQdZ5Fsz9QBNYFie/stream-input"
//             .into_client_request()
//             .unwrap();
//     request
//         .headers_mut()
//         .insert("api-key", elevenlabs_api_key_clone.parse().unwrap());
//
//     let (ws_stream, _) = connect_async(request).await.unwrap();
//     let (mut write, read) = ws_stream.split();
//
//     while let Some(ev) = ws_rx.recv().await {
//         let body = ElevenLabsSendTextMessage { text: ev };
//         let json_str = serde_json::to_string(&body)?;
//         write.send(Message::Text(json_str.into())).await?;
//     }
//
//     Ok::<(), anyhow::Error>(())
// });
