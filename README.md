# Championship Soccer World Editor

Applicazione Rust per modificare i mondi di Championship Soccer.

## Stato attuale

Prototipo grafico con egui/eframe: tab Nations, Leagues, Cups, Teams, Players
e Transfers. La tab Leagues permette di modificare e salvare i campionati; le altre
sezioni restano prototipi. La scrittura avviene solo premendo Salva.

Leagues presenta tre colonne: paesi, divisioni ordinate per livello e dettagli.
Nome, livello, reputazione, promozioni e retrocessioni sono modificabili nella
divisione selezionata. L'ordine delle squadre cambia solo con i comandi espliciti. I controlli
segnalano nomi vuoti, livelli duplicati e incongruenze tra promozioni e retrocessioni.
Scarta modifiche ripristina tutte le divisioni dopo conferma. Con modifiche
pendenti occorre salvare o scartare prima di aprire un altro mondo o chiudere.
Salva modifiche e File > Salva scrivono soltanto i file lega modificati.

Selezionando una squadra si abilitano Sposta su, Sposta giu e Scambia squadre.
Ai confini della divisione lo spostamento scambia il club con l'ultimo della
divisione superiore o il primo di quella inferiore, individuate dal livello.
La selezione segue il club. Livelli mancanti, duplicati o divisioni vuote
impediscono uno scambio ambiguo. Scambia squadre mostra solo club dello stesso
paese. Ogni operazione conserva il numero di club; aggiunta e rimozione non
sono ancora implementate. Le modifiche restano in memoria fino a Salva.

La finestra parte massimizzata, con dimensione di ripristino 1600 x 1000.
Il testo usa Consolas installato su Windows, oppure il monospace incorporato
quando il font non e disponibile. Da Impostazioni si puo regolare la dimensione
da 14 a 32 punti (20 iniziali). La preferenza viene salvata nello storage locale
di eframe immediatamente dopo ogni modifica, oltre che alla chiusura,
separatamente dai dati del gioco. Il tema fisso blu notte usa testo bianco
neutro; il monospace e i margini tra le righe rimangono coerenti cambiando
dimensione o tema di Windows.

Da File > Apri mondo si seleziona la cartella del gioco/pacchetto contenente
Data/League, oppure direttamente Data o League. Sono supportate cartelle locali,
non ancora archivi ZIP. Il selettore nativo usa rfd.

Leagues offre ricerca per paese o nome del file, divisioni espandibili, squadre
nell'ordine originale e proprieta di promozione, retrocessione e reputazione.
Il lettore accetta UTF-8 e Windows-1252 (tramite encoding_rs). Le segnalazioni
includono il file e, per gli errori di sintassi, la riga interessata.
Dal file internazionale del mondo aperto vengono letti paesi, club di riserva,
reputazione e percorsi degli archivi nomi. In Leagues i paesi senza campionato
compaiono in un elenco separato; per le leghe esistenti i club internazionali
sono consultabili in un riquadro espandibile. Titolo e ricerca restano fissi.
Non viene ancora cercato il file internazionale originale quando manca nel pacchetto.
Le regole delle coppe internazionali restano riservate alla futura sezione Cups;
i blocchi coppa nazionali sono separati dalle divisioni, senza interpretarli
come campionati. Un errore di apertura non sostituisce il mondo gia caricato.

Esporta resta disabilitato. Transfers mostra due pannelli affiancati.

I nomi mostrati usano display di Nationalities.txt, associato tramite sezione,
names o alias. Il file viene cercato nel Data del mondo aperto, poi nel Data
del progetto del gioco. In assenza di corrispondenza si usa il nome del file
lega senza estensione; per i paesi senza lega resta il nome internazionale.
Entrambi gli elenchi sono ordinati per nome visualizzato; i riferimenti dei
file e l'ordine delle squadre non vengono modificati dalla sola visualizzazione.

Le bandiere PNG si trovano in editor/flags e sono caricate all'avvio.
Si cerca il nome del file lega (o del paese internazionale), poi il nome
visualizzato, infine NoFlag.png. Le immagini mantengono le proporzioni e
seguono l'altezza del testo in uno spazio uniforme. Riavviare dopo aver
aggiunto nuove bandiere; riaprire il mondo dopo aver modificato Nationalities.txt.

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

## Salvataggio delle leghe

Prima della scrittura vengono validati tutti i file modificati: nomi, livelli,
coerenza delle promozioni/retrocessioni, club duplicati e limite 3-24 squadre.
Sono supportati i parametri delle divisioni esistenti e gli scambi tra squadre;
creazione di divisioni e aggiunta/rimozione di club restano passi successivi.
Il salvataggio preserva coppe, commenti, righe non modificate e codifica originale
UTF-8 o Windows-1252. I caratteri non rappresentabili vengono segnalati.

Un confronto con i byte caricati impedisce di sovrascrivere modifiche esterne.
Non vengono create copie di backup automatiche. Un file temporaneo viene scritto
nella cartella superiore a League e poi rinominato sull'originale. Non vengono
lasciati file ausiliari in League, perche il gioco puo caricarli come campionati.
I backup del database sono gestiti manualmente dall'utente.
In caso di errore durante il salvataggio di piu file, quelli gia salvati sono
indicati; gli altri rimangono modificati in memoria e possono essere ritentati.
Scarta modifiche torna all'ultimo stato salvato di ciascuna lega.

Il file internazionale non viene sincronizzato da questo comando.

1. Modificare la reputazione oppure scambiare due club, poi premere Salva modifiche.
2. Riaprire il mondo per verificare la modifica; per provarla nel gioco avviare
   una nuova carriera con la lega selezionata.
