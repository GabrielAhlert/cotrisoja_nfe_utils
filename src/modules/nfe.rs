use std::{
    env,
    path::Path,
    process::Command,
};

const DEFAULT_EDITOR: &str = "notepad";              // fallback
const POST_EDIT_DIR: &str = r#"\\192.0.0.221\LeituraNFe\-fil"#;    // onde vai parar o arquivo editado



pub fn get_nfe(prefix: &str, dest_dir: &str, search_dir: &str, editor: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        fs::{self, File},
        io::{BufRead, BufReader, Read},
        path::{Path, PathBuf},
        time::SystemTime,
    };
    
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

            if editor {
                edit_and_move(&dest_path)?;
            }

            return Ok(());
        }else{
            println!("⚠️ Não é essa {core} !");
        }
    }

    println!(r#"❌ Nao encontrado!"#);
    Ok(())
}

/// Abre o `path` no editor e, se o usuário salvar/fechar, move para POST_EDIT_DIR.
pub fn edit_and_move(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // ─── 1. Descobre qual editor usar ────────────────────────────────────────
    let editor = env::var("EDITOR").unwrap_or_else(|_| DEFAULT_EDITOR.into());

    // ─── 2. Spawna o editor e espera terminar ────────────────────────────────
    let status = Command::new(&editor)
        .arg(path)        // ex.: nano <arquivo>
        .status()?;

    if !status.success() {
        return Err(format!("editor {:?} saiu com status {status}", editor).into());
    }

    // ─── 3. Move para pasta final ────────────────────────────────────────────
    let dest = Path::new(POST_EDIT_DIR)
        .join(path.file_name().unwrap_or_default());
    std::fs::create_dir_all(POST_EDIT_DIR)?;  // garante que exista
    move_across_devices(path, &dest)?;


    println!("✅ Movido para {}", dest.display());
    Ok(())
}

use std::io::{self, ErrorKind};

fn move_across_devices(src: &Path, dst: &Path) -> io::Result<()> {
    match std::fs::rename(src, dst) {
        Ok(_) => Ok(()),                           // mesma partição → ok
        Err(e) if e.kind() == ErrorKind::CrossesDevices => {
            std::fs::copy(src, dst)?;              // 1. copia
            std::fs::remove_file(src)?;            // 2. apaga o original
            Ok(())
        }
        Err(e) => Err(e),                          // outros erros
    }
}
