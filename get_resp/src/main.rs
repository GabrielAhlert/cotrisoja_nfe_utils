use clap::{Parser, ValueEnum};
use notify::{Config, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use std::{collections::HashMap, fs, path::Path};
use tokio::sync::mpsc;

/// Caminho fixo do diretório monitorado
const PATH_STR_PRD: &str = "./teste/";
const PATH_STR_QAS: &str = "./teste/";
const PREFIXES : [&str; 3] = ["resp-nota", "resp-cancel", "resp-mdfe"];

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

    #[arg(long, value_enum, default_value_t = Ambiente::PRD)]
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
#[tokio::main]
async fn main() -> NotifyResult<()> {
    let args = Args::parse();
    let path = match args.ambient {
        Ambiente::PRD => PATH_STR_PRD,
        Ambiente::QAS => PATH_STR_QAS,
    };

    watch(path, args).await
}
