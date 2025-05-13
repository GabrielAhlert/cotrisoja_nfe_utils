use clap::Parser;

/// Modulos
mod modules;
use modules::*; 

/// Caminho fixo do diretório monitorado
const PATH_STR_PRD: &str = r#"\\192.0.0.221\LeituraNFe\-resp"#;
const PATH_STR_QAS: &str = r#"\\192.0.0.222\LeituraNFe\-resp"#;

const SEARCH_DIR_PRD: &str =
    r#"\\192.0.0.221\c$\Program Files (x86)\NFeXpress\-filproc\"#;
const SEARCH_DIR_QAS: &str =
    r#"\\192.0.0.222\c$\Program Files (x86)\NFeXpress\-filproc\"#;

const DEST_DIR: &str = r#"P:\"#;

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

    if args.editor && !args.get_nfe {
        return Err("O parâmetro --editor só pode ser usado junto com --get-nfe".into());
    }

    if args.get_nfe && args.editor{
        println!("📦 Buscando por NF-e");
            loop {
                get_nfe(&args.prefix, dest_dir, search_dir, args.editor)?;
                match watch(path, &args).await {
                    Ok(_) => break,
                    Err(e) => {
                        println!("Erro ao monitorar o diretório: {}", e);
                        continue;
                    }
                }
            }

    } else if args.get_nfe {
        println!("📦 Buscando por NF-e");
        get_nfe(&args.prefix, dest_dir, search_dir, args.editor)?;
    } else {

        watch(path, &args).await?;
    }

    Ok(())
}
