use clap::{Parser, ValueEnum};
use notify::{Config, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use std::{collections::HashMap, fs::{self, File}, io::{BufRead, BufReader, Read}, path::{Path, PathBuf}, time::SystemTime};
use tokio::sync::mpsc;

/// Caminho fixo do diretório monitorado
const PATH_STR_PRD: &str = r#"\\192.0.0.221\LeituraNFe\-resp"#;
const PATH_STR_QAS: &str = r#"\\192.0.0.222\LeituraNFe\-resp"#;

const PREFIXES : [&str; 3] = ["resp-nota", "resp-cancel", "resp-mdfe"];

const SEARCH_DIR_PRD: &str =
    r#"\\192.0.0.221\c$\Program Files (x86)\NFeXpress\-filproc\"#;
const SEARCH_DIR_QAS: &str =
    r#"\\192.0.0.222\c$\Program Files (x86)\NFeXpress\-filproc\"#;

const DEST_DIR: &str = r#"P:\"#;

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
enum Ambiente {
    #[default]
    PRD,
    QAS,
}

#[derive(Parser, Debug)]
struct Args {
    /// Prefixo do arquivo
    prefix: String,
    
    /// Programa para Buscar nota que foi rejeitada e colocar na publica
    #[arg(short, long)]
    get_nfe: bool,
    
    /// Ambiente Escolhido
    #[arg(short, long, value_enum, default_value_t = Ambiente::PRD)]
    ambient: Ambiente
}

fn pretty_print(props: &HashMap<String, String>) {
    // ordem que faz sentido para NF-e
    let order = [
        "Resultado",
        "Nprotocolo",
        "Mensagem",
        "ChaveNFe",
    ];

    println!("\n\x1b[1m========= Detalhes da NF-e 📝 ==========\x1b[0m"); // negrito ANSI
    for key in order {
        if let Some(val) = props.get(key) {
            // 15 caracteres de largura para alinhar as “labels”
            println!("{:<15}: {}", key, val);
        }
    }
    println!("=======================================\n");
}

fn read_props(path: &Path) -> std::io::Result<HashMap<String, String>> {
    let bytes = fs::read(path)?;               // lê em bytes
    let txt = String::from_utf8_lossy(&bytes);  // converte (troca bytes inválidos por �)

    let mut map = HashMap::new();
    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once('=').unwrap_or((line, ""));
        map.insert(k.to_string(), v.to_string());
    }
    Ok(map)
}


async fn watch(path: &str, args: Args) -> NotifyResult<()> {
    let (tx, mut rx) = mpsc::channel(100);

    let prefix = args.prefix;

    // Cria watcher assíncrono enviando eventos para o canal mpsc
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.blocking_send(res);
        },
        Config::default(),
    )?;

    watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
    println!("📡 Monitorando {} com prefixo '{}'", path, prefix);

    // Loop assíncrono que processa eventos

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) if event.kind.is_create() => {
                for p in event.paths {
                    if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                        let ok = PREFIXES.iter().any(|base| {
                            fname.starts_with(base) && {
                                // trecho depois do prefixo base
                                let tail = &fname[base.len()..];
                                // remove zeros à esquerda
                                let tail_no_zeros = tail.trim_start_matches('0');
                                //corta tudo após o '#'
                                let tail_core     = tail_no_zeros
                                    .split('#') 
                                    .next()          
                                    .unwrap_or("");   
                                // agora compara com o que o usuário digitou
                                tail_core.starts_with(&prefix)
                            }
                        });
    
                        if ok {
                            println!("🛈 Arquivo recebido: {fname}");
                            match read_props(&p) {
                                Ok(props) => {
                                    pretty_print(&props);
                        
                                    // ─────────── Copia ou exclui conforme Resultado ───────────
                                    match props.get("Resultado").map(String::as_str) {
                                        Some("100") | Some("124") => {
                                            println!("✅ Processado com sucesso: {fname}");
                                        }
                                        Some(outro) => {
                                            if let Err(e) = fs::remove_file(&p) {
                                                eprintln!("⚠️ Falha ao excluir {fname}: {e}");
                                            } else {
                                                println!("🗑  Excluído {fname} (Resultado={outro})\n");
                                            }
                                        }
                                        None => eprintln!("⚠️ 'Resultado' ausente em {fname}"),
                                    }
                                    // ──────────────────────────────────────────────────────────
                                }
                                Err(e) => eprintln!("⚠️ Falha ao ler {fname}: {e}"),
                            }
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("⚠️  Watcher erro: {e:?}"),
        }
    }
    Ok(())
} 

fn get_nfe(prefix: &str, dest_dir: &str, search_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // ───── 1. Coleta entradas + timestamp ────────────────────────────────────
    let mut entries: Vec<(PathBuf, SystemTime)> = fs::read_dir(search_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let created = meta.created().or_else(|_| meta.modified()).ok()?;
            Some((e.path(), created))
        })
        .collect();

    println!("{}", entries.len());

    // Ordenar decrescente e cortar nos 250 mais novos
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries.truncate(250);

    // ───── 2. Examina cada arquivo ───────────────────────────────────────────
    for (path, _) in entries {
        let mut reader = BufReader::new(File::open(&path)?);

        // Lê 1ª linha (descarta) e 2ª linha (nome)
        let mut buf = Vec::new();
        reader.read_until(b'\n', &mut buf)?; // linha 1
        buf.clear();
        reader.read_until(b'\n', &mut buf)?; // linha 2

        let name_line  = String::from_utf8_lossy(&buf);
        let name_clean = name_line.trim_end();
        let core       = name_clean.split('#').next().unwrap_or("");

        if core.contains(prefix) {
            println!("✅ Encontrado!");

            // 3. Copia para destino
            let dest_path = Path::new(dest_dir).join(name_clean);
            fs::copy(&path, &dest_path)?;

            // 4. Remove as duas primeiras linhas do arquivo copiado
            let mut bytes = Vec::new();
            File::open(&dest_path)?.read_to_end(&mut bytes)?;

            let second_nl = bytes
                .iter()
                .enumerate()
                .filter(|&(_, &b)| b == b'\n')
                .nth(1)
                .map(|(i, _)| i + 1)
                .unwrap_or(0);

            fs::write(&dest_path, &bytes[second_nl..])?;
            return Ok(());
        }else{
            println!("⚠️ Não é essa {core} !");
        }
    }

    println!(r#"⚠️ Nao encontrado!"#);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let path = match args.ambient {
        Ambiente::PRD => PATH_STR_PRD,
        Ambiente::QAS => PATH_STR_QAS,
    };

    let dest_dir = match args.ambient {
        Ambiente::PRD => DEST_DIR,
        Ambiente::QAS => DEST_DIR,
    };

    let search_dir = match args.ambient {
        Ambiente::PRD => SEARCH_DIR_PRD,
        Ambiente::QAS => SEARCH_DIR_QAS,
    };

    if args.get_nfe {
        println!("📦 Buscando por NF-e");
        get_nfe(&args.prefix, dest_dir, search_dir)?;             // ← usa o prefixo informado
    } else {
        // `watch` devolve `notify::Error`; `?` faz a conversão automática
        watch(path, args).await?;
    }

    Ok(())
}
