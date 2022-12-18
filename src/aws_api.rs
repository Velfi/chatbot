use aws_sdk_transcribe::model::{LanguageCode, Media};
use tracing::instrument;

#[instrument]
pub async fn create_transcription_bucket(client: &aws_sdk_s3::Client) -> Result<(), anyhow::Error> {
    let _ = client
        .create_bucket()
        .bucket("")
        .send()
        .await?;

    Ok(())
}

#[instrument]
pub async fn upload_speech(
    client: &aws_sdk_s3::Client,
    speech_file: (),
) -> Result<(), anyhow::Error> {
    let _ = client
        .put_object()
        .bucket("")
        .key("")
        .body(speech_file)
        .send()
        .await?;

    Ok(())
}

#[instrument]
pub async fn speech_to_text(
    client: &aws_sdk_transcribe::Client,
    speech_file: (),
) -> Result<(), anyhow::Error> {
    let media = Media::builder()
        .media_file_uri("")
        .build();
    let _ = client
        .start_transcription_job()
        .language_code(LanguageCode::EnUs)
        .media(media)
        .output_bucket_name("")
        .output_key("")

        .send()
        .await?;

    todo!()
}

#[instrument]
pub async fn text_to_speech(
    client: &aws_sdk_polly::Client,
    text: &str,
) -> Result<(), anyhow::Error> {
    let speech = client.synthesize_speech().text(text).send().await?;

    Ok(())
}
