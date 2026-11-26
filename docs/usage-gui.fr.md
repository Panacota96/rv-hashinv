> La version de référence est la version anglaise ([usage-gui.md](usage-gui.md)) ; cette traduction la suit.

# Utiliser l'application de bureau (Windows 10/11)

Lancez **RV HashInv** depuis le menu Démarrer (programme d'installation) ou double-cliquez sur `rv-hashinv-gui.exe`
(zip portable). Aucun autre logiciel n'est nécessaire ; l'application fonctionne aussi sur des machines virtuelles
sans pilote graphique (rendu logiciel).

![Fenêtre principale](images/gui-main.png)

## Déroulement

```mermaid
flowchart TD
    A["1 · Éléments de preuve<br/>Ajouter des fichiers… / Ajouter un dossier… / glisser-déposer / saisir un chemin"] --> B
    B["2 · Algorithmes d'empreinte<br/>cochez 1 ou 2 (une troisième case est désactivée)"] --> C
    C["3 · Rapports<br/>langue · dossier de sortie · expert · référence du dossier"] --> D
    D["Lancer l'analyse"] --> E{"En cours"}
    E -->|barre de progression, écoulé, restant| E
    E -->|Annuler| F["Rapports partiels + run_cancelled dans le journal"]
    E -->|terminé| G["Ouvrir le rapport · Ouvrir le dossier des journaux"]
```

## Examiner des éléments en 6 étapes

1. **Ajoutez les éléments.** *Ajouter des fichiers…* ou *Ajouter un dossier…*, glissez des fichiers sur la fenêtre,
   ou saisissez un chemin et cliquez sur *Ajouter le chemin*. Retirez une entrée avec **x**. Rien n'est encore
   écrit : chaque modification est gardée sous `ui_input_added` / `ui_input_removed` et entre dans le journal
   d'audit au lancement de l'exécution.
2. **Choisissez les algorithmes d'empreinte.** Cochez-en un ou deux (gardez `sha256`). Gardé sous
   `ui_hash_selection_changed`.
3. **Réglez les rapports.** *Langue des rapports*, *Dossier de sortie* (ou un **Dossier de l'affaire**, voir plus
   bas), *Expert* et *Référence du dossier*. Gardés sous `ui_language_changed` et `ui_case_dir_changed` ; l'expert
   est consigné comme `actor` de chaque entrée d'audit et la référence dans `run_started` ; tous deux sont imprimés
   sur chaque rapport et fiche de traçabilité.
4. **Réglez les options de confiance.** *Fiche de possession* (Colombie / France), *Horodatage qualifié* et
   l'adresse de sa TSA, *Certificat de signature (.p12/.pfx)* et *Mot de passe du certificat*. Gardés sous
   `ui_jurisdiction_changed`, `ui_timestamp_changed` et `ui_signing_certificate_changed` (le chemin seulement ;
   le mot de passe n'est jamais journalisé).
5. **Cliquez sur *Lancer l'analyse*.** L'exécution écrit `<OUT>/logs/<RUN-ID>/` (journal d'audit, registre des
   empreintes, environnement, jeton d'horodatage) et `<OUT>/reports/<RUN-ID>/` (statistiques et un dossier par
   élément avec l'analyse technique, la synthèse et la fiche de traçabilité). *Annuler* s'arrête après le fichier
   en cours et écrit des rapports partiels.
6. **Ouvrez les résultats.** **Ouvrir le rapport** ouvre la page des statistiques dans la langue des rapports (en
   anglais si les trois sont choisies) ; **Ouvrir le dossier des journaux** ouvre `<OUT>/logs/<RUN-ID>/`. Lisez
   le nombre par gravité et les **Avertissements** avant de vous fonder sur l'exécution.

## La fenêtre

| Zone | Ce qu'elle fait |
| --- | --- |
| **1. Éléments de preuve** | *Ajouter des fichiers…* et *Ajouter un dossier…* ouvrent les sélecteurs de Windows. Vous pouvez aussi glisser des fichiers ou des dossiers sur la fenêtre (ou sur l'icône de `rv-hashinv-gui.exe` avant son lancement), ou coller un chemin et appuyer sur Entrée. *Inclure les sous-dossiers* est coché par défaut. Retirez une entrée avec **x**. |
| **2. Algorithmes d'empreinte** | Cochez-en un ou deux. Quand deux sont cochés, les autres sont désactivés ; on ne peut pas décocher le dernier. |
| **3. Rapports** | Langue (English, Español, Français, ou les trois), dossier de sortie, nom de l'expert (par défaut votre utilisateur Windows), référence du dossier facultative. |
| **4. Confiance** | *Fiche de possession* : Colombie (Fiscalía) ou France (scellé), pour la fiche de traçabilité. *Horodatage qualifié* (activé par défaut ; adresse de la TSA modifiable) : malgré son libellé, cet horodatage n'est qualifié au sens d'eIDAS que si l'adresse est celle d'un prestataire de services de confiance qualifié (QTSP) inscrit sur une liste de confiance de l'UE ; l'adresse par défaut (DigiCert) ne l'est pas (*to be verified*). *Certificat de signature (.p12/.pfx)* (saisissez un chemin ou cliquez sur *Parcourir…*) et *Mot de passe du certificat* (gardé en mémoire seulement, jamais journalisé) afin de signer chaque PDF. |
| **Langue de l'interface** | Sélecteur en haut à droite : English, Español, Français. Change immédiatement tous les libellés ; indépendant de la langue des rapports. |
| **Lancer l'analyse / Annuler** | Lancer est actif dès qu'un élément est ajouté. Pendant une exécution, les réglages sont verrouillés et le bouton devient *Annuler*. |
| **Progression** | Barre selon les octets traités, `fichiers traités / total`, temps écoulé (toujours en mouvement) et temps restant estimé. Le fichier en cours de lecture s'affiche en dessous. |
| **Résultat** | Nombre par gravité, **Ouvrir le rapport** (page des statistiques dans votre langue) et **Ouvrir le dossier des journaux**. |
| **Avis de version** | Au démarrage, l'application vérifie une fois, en arrière-plan, si une version plus récente est publiée (une seule requête HTTPS GET de la page publique des versions sur GitHub, avec seulement `User-Agent: rv-hashinv/<version>`). Si c'est le cas, un bandeau la nomme avec un lien vers sa page ; rien n'est téléchargé ni installé, et la vérification n'est écrite dans aucun journal d'audit. Désactivez-la avec `RV_NO_UPDATE_CHECK=1` ou le fichier `update-check-off` dans `%APPDATA%\Rakvor` ([installation.fr.md](installation.fr.md#vérification-des-versions)). |

## Dossier de l'affaire

Renseignez **Dossier de l'affaire** dans la section 1 pour travailler dans un dossier partagé de l'affaire (voir
[case-folder.md](case-folder.md), en anglais). Pour un nouveau dossier, renseignez aussi la **Référence du dossier**
(et la **Fiche de possession**) : elles sont écrites une seule fois dans `case.json`. Tant qu'un dossier
de l'affaire est défini, le **Dossier de sortie** affiche `<case>/runs/rv-hashinv/<RUN-ID>` et n'est pas modifiable,
et vous pouvez lancer sans ajouter d'élément (chaque élément enregistré est examiné). Après l'exécution, le panneau de
résultat liste l'**ID d'élément** de chaque élément (E0001…) et son statut (enregistré, vérifié, divergent, absent du
registre). La modification est auditée sous `ui_case_dir_changed`.

RV Hub ouvre l'application avec le dossier de l'affaire déjà renseigné : `rv-hashinv-gui.exe --case-dir <CASE> [FILES...]`
(ou `--case-dir=<CASE>`). Le journal d'audit commence alors par `ui_case_dir_from_launcher` (le chemin et `at_utc`) ;
un `--case-dir` sans dossier est ignoré, signalé dans la fenêtre et journalisé sous `ui_launch_argument_ignored`.

## Ce que la GUI consigne

Vos choix font partie de la trace des éléments de preuve. Avant le début de l'exécution, la GUI les inscrit dans le
journal d'audit :

| Action | Consignée sous |
| --- | --- |
| Ajouter / retirer un élément | `ui_input_added`, `ui_input_removed` |
| Modifier la sélection d'empreintes | `ui_hash_selection_changed` |
| Modifier la langue des rapports / les sous-dossiers | `ui_language_changed`, `ui_recursive_changed` |
| Modifier la langue de l'interface | `ui_interface_language_changed` |
| Modifier la fiche de possession | `ui_jurisdiction_changed` |
| Modifier l'horodatage (activé/désactivé, adresse de la TSA) | `ui_timestamp_changed` |
| Modifier le certificat de signature (le chemin ; jamais le mot de passe) | `ui_signing_certificate_changed` |
| Ouverte par RV Hub avec `--case-dir` | `ui_case_dir_from_launcher`, `ui_case_dir_changed` (ou `ui_launch_argument_ignored`) |
| Clic sur Lancer | `ui_start_clicked` (réglages finaux) |
| Clic sur Annuler | `item_cancelled`, `run_cancelled` |

Les erreurs pendant l'exécution apparaissent sous **Avertissements** et dans le journal sous `item_error`.
Les messages de validation affichés *avant* une exécution (p. ex. « Add at least one file or folder ») ne sont pas
journalisés, car aucune exécution — et donc aucun journal — n'existe encore.

## Mêmes résultats que la ligne de commande

La GUI appelle le même moteur que `rv-hashinv scan`. Une exécution dans la GUI avec les mêmes éléments, les mêmes
algorithmes et la même langue donne des empreintes et des constatations identiques ; `environment.json` consigne
`"frontend": "gui"` et les réglages utilisés.
