use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
pub enum Ambiente {
    #[default]
    PRD,
    QAS,
}

#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Prefixo do arquivo
    pub prefix: String,
    
    /// Programa para Buscar nota que foi rejeitada e colocar na publica
    #[arg(short, long)]
    pub get_nfe: bool,
    
    /// Ambiente Escolhido
    #[arg(short, long, value_enum, default_value_t = Ambiente::PRD)]
    pub ambient: Ambiente,

    #[arg(short, long)]
    pub editor: bool
}