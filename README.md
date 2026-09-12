# Championship Soccer World Editor

Applicazione Rust per modificare i mondi di Championship Soccer.

## Stato attuale

Scheletro eseguibile senza dipendenze esterne. Non carica e non modifica ancora
i dati del gioco. La libreria per l'interfaccia grafica resta da scegliere.

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

Il prossimo micro-step previsto e il caricamento in sola lettura dei file lega.
