# Gestionnaire de Notes

Application desktop de gestion de notes développée en Rust avec egui/eframe.

## Fonctionnalités

- Créer, modifier et supprimer des notes
- Filtrage par titre/contenu et par tag
- Persistance JSON ou SQLite (bascule à chaud)
- Import de notes depuis une API REST (jsonplaceholder)
- Export JSON
- Dashboard (nombre de notes, dernière modification)
- Thème clair / sombre
- Raccourcis clavier : `Ctrl+N` nouvelle note, `Ctrl+S` sauvegarder, `Échap` désélectionner

## Prérequis

- [Rust](https://rustup.rs/) 1.70+
- Visual Studio Build Tools 2022 (composant C++ — requis sur Windows)

## Lancer le projet

```bash
# Cloner le dépôt
git clone https://github.com/ton-compte/Projet-Gestionnaire-Note.git
cd Projet-Gestionnaire-Note/projet-final-notes

# Compiler et lancer
cargo run

# Compiler en mode release
cargo build --release
# L'exécutable se trouve dans target/release/
```

## Structure

```
projet-final-notes/
├── Cargo.toml
└── src/
    ├── main.rs    # Point d'entrée
    ├── model.rs   # Structures de données (Note, AppState)
    ├── dao.rs     # Persistance JSON et SQLite
    ├── api.rs     # Import REST
    └── app.rs     # Interface graphique (egui)
```

---

**Axel Perrin** — Projet B3 Dev Desktop