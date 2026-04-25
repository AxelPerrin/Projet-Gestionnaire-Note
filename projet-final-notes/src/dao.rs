use crate::model::Note;
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Trait définissant les opérations CRUD sur les notes.
pub trait Dao {
    fn lister(&self) -> Result<Vec<Note>, String>;
    fn creer(&self, note: &Note) -> Result<(), String>;
    fn mettre_a_jour(&self, note: &Note) -> Result<(), String>;
    fn supprimer(&self, id: Uuid) -> Result<(), String>;
}

// ─── JSON DAO ────────────────────────────────────────────────────────────────

pub struct JsonDao {
    chemin: PathBuf,
}

impl JsonDao {
    pub fn nouveau(chemin: PathBuf) -> Self {
        Self { chemin }
    }

    fn lire_notes(&self) -> Result<Vec<Note>, String> {
        if !self.chemin.exists() {
            return Ok(Vec::new());
        }
        let contenu = fs::read_to_string(&self.chemin)
            .map_err(|e| format!("Lecture JSON échouée : {e}"))?;
        serde_json::from_str(&contenu).map_err(|e| format!("Désérialisation JSON échouée : {e}"))
    }

    fn ecrire_notes(&self, notes: &[Note]) -> Result<(), String> {
        if let Some(parent) = self.chemin.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Création dossier échouée : {e}"))?;
        }
        let json =
            serde_json::to_string_pretty(notes).map_err(|e| format!("Sérialisation échouée : {e}"))?;
        fs::write(&self.chemin, json).map_err(|e| format!("Écriture JSON échouée : {e}"))
    }
}

impl Dao for JsonDao {
    fn lister(&self) -> Result<Vec<Note>, String> {
        self.lire_notes()
    }

    fn creer(&self, note: &Note) -> Result<(), String> {
        let mut notes = self.lire_notes()?;
        notes.push(note.clone());
        self.ecrire_notes(&notes)
    }

    fn mettre_a_jour(&self, note: &Note) -> Result<(), String> {
        let mut notes = self.lire_notes()?;
        match notes.iter_mut().find(|n| n.id == note.id) {
            Some(existante) => *existante = note.clone(),
            None => return Err(format!("Note {} introuvable", note.id)),
        }
        self.ecrire_notes(&notes)
    }

    fn supprimer(&self, id: Uuid) -> Result<(), String> {
        let mut notes = self.lire_notes()?;
        let avant = notes.len();
        notes.retain(|n| n.id != id);
        if notes.len() == avant {
            return Err(format!("Note {id} introuvable"));
        }
        self.ecrire_notes(&notes)
    }
}

// ─── SQLITE DAO ──────────────────────────────────────────────────────────────

pub struct SqliteDao {
    chemin: PathBuf,
}

impl SqliteDao {
    pub fn nouveau(chemin: PathBuf) -> Result<Self, String> {
        let dao = Self { chemin };
        dao.initialiser_schema()?;
        Ok(dao)
    }

    fn ouvrir_connexion(&self) -> Result<Connection, String> {
        if let Some(parent) = self.chemin.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Création dossier échouée : {e}"))?;
        }
        Connection::open(&self.chemin).map_err(|e| format!("Connexion SQLite échouée : {e}"))
    }

    fn initialiser_schema(&self) -> Result<(), String> {
        let conn = self.ouvrir_connexion()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                id          TEXT PRIMARY KEY,
                titre       TEXT NOT NULL,
                contenu     TEXT NOT NULL,
                tags        TEXT NOT NULL,
                cree_le     TEXT NOT NULL,
                modifie_le  TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Création table échouée : {e}"))
    }
}

impl Dao for SqliteDao {
    fn lister(&self) -> Result<Vec<Note>, String> {
        let conn = self.ouvrir_connexion()?;
        let mut stmt = conn
            .prepare("SELECT id, titre, contenu, tags, cree_le, modifie_le FROM notes ORDER BY modifie_le DESC")
            .map_err(|e| format!("Préparation requête échouée : {e}"))?;

        let notes = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let tags_str: String = row.get(3)?;
                let cree_str: String = row.get(4)?;
                let modif_str: String = row.get(5)?;
                Ok((id_str, row.get::<_, String>(1)?, row.get::<_, String>(2)?, tags_str, cree_str, modif_str))
            })
            .map_err(|e| format!("Requête échouée : {e}"))?
            .map(|r| {
                r.map_err(|e| format!("Ligne invalide : {e}")).and_then(
                    |(id_str, titre, contenu, tags_str, cree_str, modif_str)| {
                        let id = Uuid::parse_str(&id_str)
                            .map_err(|e| format!("UUID invalide : {e}"))?;
                        let tags: Vec<String> = serde_json::from_str(&tags_str)
                            .map_err(|e| format!("Tags JSON invalides : {e}"))?;
                        let cree_le = cree_str
                            .parse()
                            .map_err(|e| format!("Date cree_le invalide : {e}"))?;
                        let modifie_le = modif_str
                            .parse()
                            .map_err(|e| format!("Date modifie_le invalide : {e}"))?;
                        Ok(Note { id, titre, contenu, tags, cree_le, modifie_le })
                    },
                )
            })
            .collect::<Result<Vec<Note>, String>>()?;

        Ok(notes)
    }

    fn creer(&self, note: &Note) -> Result<(), String> {
        let conn = self.ouvrir_connexion()?;
        let tags_json = serde_json::to_string(&note.tags)
            .map_err(|e| format!("Sérialisation tags : {e}"))?;
        conn.execute(
            "INSERT INTO notes (id, titre, contenu, tags, cree_le, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                note.id.to_string(),
                note.titre,
                note.contenu,
                tags_json,
                note.cree_le.to_rfc3339(),
                note.modifie_le.to_rfc3339(),
            ],
        )
        .map_err(|e| format!("Insertion SQLite échouée : {e}"))?;
        Ok(())
    }

    fn mettre_a_jour(&self, note: &Note) -> Result<(), String> {
        let conn = self.ouvrir_connexion()?;
        let tags_json = serde_json::to_string(&note.tags)
            .map_err(|e| format!("Sérialisation tags : {e}"))?;
        let modifies = conn
            .execute(
                "UPDATE notes SET titre=?1, contenu=?2, tags=?3, modifie_le=?4 WHERE id=?5",
                params![
                    note.titre,
                    note.contenu,
                    tags_json,
                    note.modifie_le.to_rfc3339(),
                    note.id.to_string(),
                ],
            )
            .map_err(|e| format!("Mise à jour SQLite échouée : {e}"))?;
        if modifies == 0 {
            return Err(format!("Note {} introuvable", note.id));
        }
        Ok(())
    }

    fn supprimer(&self, id: Uuid) -> Result<(), String> {
        let conn = self.ouvrir_connexion()?;
        let modifies = conn
            .execute("DELETE FROM notes WHERE id=?1", params![id.to_string()])
            .map_err(|e| format!("Suppression SQLite échouée : {e}"))?;
        if modifies == 0 {
            return Err(format!("Note {id} introuvable"));
        }
        Ok(())
    }
}
