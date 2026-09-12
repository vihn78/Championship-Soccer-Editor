# Championship Soccer World Editor

Applicazione Rust per modificare i mondi di Championship Soccer.

## Stato attuale

Prototipo grafico con egui/eframe: tab Nations, Leagues, Cups, Teams, Players
e Transfers. La tab Leagues carica i campionati in sola lettura; le altre
sezioni restano prototipi. Nessun file del mondo viene modificato.

La finestra parte massimizzata, con dimensione di ripristino 1600 x 1000.
Il testo usa Consolas installato su Windows, oppure il monospace incorporato
quando il font non e disponibile. Da Impostazioni si puo regolare la dimensione
da 14 a 32 punti (20 iniziali). La preferenza viene salvata nello storage locale
di eframe alla chiusura e periodicamente, separatamente dai dati del gioco.

Da File > Apri mondo si seleziona la cartella del gioco/pacchetto contenente
Data/League, oppure direttamente Data o League. Sono supportate cartelle locali,
non ancora archivi ZIP. Il selettore nativo usa rfd.

Leagues offre ricerca per paese o nome del file, divisioni espandibili, squadre
nell'ordine originale e proprieta di promozione, retrocessione e reputazione.
Il lettore accetta UTF-8 e Windows-1252 (tramite encoding_rs). Le segnalazioni
includono il file e, per gli errori di sintassi, la riga interessata.
Il file internazionale e riconosciuto e riservato alla futura sezione Cups;
i blocchi coppa nazionali sono separati dalle divisioni, senza interpretarli
come campionati. Un errore di apertura non sostituisce il mondo gia caricato.

Salva ed Esporta restano disabilitati. Transfers mostra due pannelli affiancati.

## Sviluppo

Dalla cartella `editor`:

```powershell
cargo run
cargo test
```

Controlli di formattazione e analisi statica:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Quando l'applicazione sara completa, la build finale si produrra con
`cargo build --release`.

## Regole del progetto

- Procedere per micro-step e preferire modifiche minime e leggibili.
- Pianificare i task complessi prima di scrivere codice.
- Chiedere prima di creare nuovi file o aggiungere dipendenze.
- Usare nomi espliciti e commenti che chiariscano le regole del gioco.
- Verificare prima la correttezza; ottimizzare in seguito sulla base di misure.
- Conservare `Cargo.lock` per rendere riproducibile la risoluzione delle dipendenze.

## Regole del mondo concordate

- Una lega selezionata nella carriera ha precedenza sui club del relativo
  paese elencati nel file internazionale. Se non e selezionata, il gioco usa
  l'elenco internazionale.
- L'editor sincronizzera l'elenco internazionale con le prime N squadre della
  divisione 1. N andra ricavato dalle regole delle coppe, tenendo conto di
  condizioni e alternative senza sommare indiscriminatamente le istruzioni.
- Creando una lega, i club gia presenti nel blocco internazionale saranno
  importati una sola volta e resteranno modificabili, spostabili ed eliminabili.
- Ogni giocatore avra un file dedicato in `Data/Player`; i file squadra
  conterranno i riferimenti della rosa e i ruoli.
- In lettura, un file individuale prevale integralmente sui dettagli incorporati
  nel file squadra: le due fonti non vengono unite.
- Un trasferimento aggiornera la vecchia e la nuova rosa mantenendo il file
  individuale del giocatore.

## Riferimenti

- [Editing dei dati](https://championshipsoccer.net/manual/data-editing.html)
- [Pacchetti personalizzati](https://championshipsoccer.net/manual/custom-football-worlds.html)

## Verifica del caricamento

1. Avviare con `cargo run`, scegliere File > Apri mondo e selezionare la cartella
   del gioco.
2. In Leagues cercare Italy: devono comparire nove divisioni; Serie A contiene
   venti squadre, con Inter, Napoli e Roma nelle prime tre posizioni.

`cargo test` include test del parser e una verifica sui dati del gioco presenti
accanto alla cartella editor; quest'ultima richiede il dataset originale corrente.
