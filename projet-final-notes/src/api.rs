use crate::model::Note;
use std::sync::mpsc::Sender;

/// Structure de réponse de l'API JSONPlaceholder.
#[derive(serde::Deserialize)]
struct Post {
    id: u32,
    title: String,
    body: String,
}

/// Lance un thread pour récupérer des notes depuis l'API REST.
/// Le résultat est envoyé via le canal `tx`.
pub fn fetch_sample_notes(tx: Sender<Result<Vec<Note>, String>>) {
    std::thread::spawn(move || {
        let resultat = telecharger_notes();
        // On ignore l'erreur d'envoi si le récepteur a été supprimé.
        let _ = tx.send(resultat);
    });
}

fn telecharger_notes() -> Result<Vec<Note>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Création client HTTP : {e}"))?;

    let posts: Vec<Post> = client
        .get("https://jsonplaceholder.typicode.com/posts")
        .send()
        .map_err(|e| format!("Requête HTTP échouée : {e}"))?
        .json()
        .map_err(|e| format!("Désérialisation réponse : {e}"))?;

    let notes = posts
        .into_iter()
        .take(20) // On limite à 20 pour ne pas surcharger l'UI
        .map(|post| {
            Note::nouveau(
                format!("[{}] {}", post.id, post.title),
                post.body,
                vec!["import".to_string()],
            )
        })
        .collect();

    Ok(notes)
}
