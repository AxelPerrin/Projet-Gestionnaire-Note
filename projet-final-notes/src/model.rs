use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Représente une note avec ses métadonnées.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub titre: String,
    pub contenu: String,
    pub tags: Vec<String>,
    pub cree_le: DateTime<Utc>,
    pub modifie_le: DateTime<Utc>,
}

impl Note {
    pub fn nouveau(titre: impl Into<String>, contenu: impl Into<String>, tags: Vec<String>) -> Self {
        let maintenant = Utc::now();
        Self {
            id: Uuid::new_v4(),
            titre: titre.into(),
            contenu: contenu.into(),
            tags,
            cree_le: maintenant,
            modifie_le: maintenant,
        }
    }
}

/// Backend de persistance sélectionné.
#[derive(Debug, Clone, PartialEq)]
pub enum Backend {
    Json,
    Sqlite,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Json => write!(f, "JSON"),
            Backend::Sqlite => write!(f, "SQLite"),
        }
    }
}

/// État global de l'application.
pub struct AppState {
    pub notes: Vec<Note>,
    pub filtre: String,
    pub tag_filtre: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            filtre: String::new(),
            tag_filtre: None,
        }
    }
}

impl AppState {
    /// Retourne les notes correspondant au filtre texte et au filtre tag.
    pub fn notes_filtrees(&self) -> Vec<&Note> {
        let filtre_lower = self.filtre.to_lowercase();
        self.notes
            .iter()
            .filter(|n| {
                let correspond_texte = filtre_lower.is_empty()
                    || n.titre.to_lowercase().contains(&filtre_lower)
                    || n.contenu.to_lowercase().contains(&filtre_lower);
                let correspond_tag = match &self.tag_filtre {
                    None => true,
                    Some(tag) => n.tags.iter().any(|t| t == tag),
                };
                correspond_texte && correspond_tag
            })
            .collect()
    }

    /// Retourne tous les tags uniques présents dans les notes.
    pub fn tous_les_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .notes
            .iter()
            .flat_map(|n| n.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }

    /// Retourne la date de dernière modification parmi toutes les notes.
    pub fn derniere_modification(&self) -> Option<DateTime<Utc>> {
        self.notes.iter().map(|n| n.modifie_le).max()
    }
}
