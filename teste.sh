#!/usr/bin/env fish

# Diretórios envolvidos
set SRC_DIR teste_exemplo
set DST_DIR teste

# Garante que o diretório de destino exista
mkdir -p $DST_DIR

# Loop infinito
while true
    echo "📂 Copiando arquivos de '$SRC_DIR' para '$DST_DIR'..."
    cp $SRC_DIR/* $DST_DIR/ 2>/dev/null

    echo "⏳ Aguardando 10 segundos..."
    sleep 10

    echo "🗑  Removendo arquivos de '$DST_DIR'..."
    rm -f $DST_DIR/*

    # opcional: uma pausa antes de reiniciar
    # sleep 1
end
