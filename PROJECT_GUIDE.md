# Project L - Run and Use Guide

Este ficheiro contém o fluxo completo: instalar, correr e usar a aplicação (editor + OLM).

## 0) Documentação de contexto técnico

Para implementação/manutenção com mapa de funções e ficheiros, consultar:

- `IMPLEMENTATION_CONTEXT.md`

## 1) Pré-requisitos

- Node.js 18+ (recomendado: LTS recente)
- Rust toolchain (stable) via `rustup`
- Dependências de build para Tauri no Windows (WebView2 e Visual Studio Build Tools)

## 2) Instalar dependências

Na raiz do projeto:

```bash
npm install
```

## 3) Correr o projeto

### 3.1 Modo frontend (rápido para UI)

```bash
npm run dev
```

### 3.2 Modo app desktop Tauri (recomendado)

```bash
npm run tauri dev
```

## 4) Como usar (fluxo principal)

### 4.1 Abrir pasta de estudo

1. Clica em **Escolher Pasta** na sidebar.
2. Seleciona a pasta root das tuas notas.
3. A árvore de ficheiros aparece na esquerda.

### 4.2 Editar notas

1. Clica num ficheiro na árvore para abrir.
2. Edita no editor central.
3. Clica em **Guardar** para persistir alterações.

### 4.3 OLM (painel direito)

O painel **OLM** permite gerir o modelo de aprendizagem:

- **Novo conceito**: cria um conceito manualmente (`id` + `nome`).
- **Pré-requisito**: liga conceitos (prereq -> alvo).
- **Evento**: injeta eventos manuais (`practice_attempt`, `quiz_attempt`, etc.).
- **Estado**: mostra `mastery` e `uncertainty` por conceito.
- **Next to study**: recomendações com justificações.
- **Explicação**: rasto de evidências por conceito.

## 5) Eventos automáticos já integrados

Sem ação extra do utilizador:

- Ao **abrir um ficheiro**, é registado um evento `review`.
- Ao **guardar um ficheiro**, não é injetado evento automático por omissão.

Os eventos automáticos usam o conceito do **perfil do nó** (ficheiro/pasta/domínio).

Também é possível declarar conceitos diretamente no conteúdo usando `;;;nome do conceito;;;`.
Esses conceitos são normalizados para IDs namespaced por domínio e entram no fluxo OLM.

## 6) Validação rápida

### Frontend build

```bash
npm run build
```

### Backend Rust check

```bash
cd src-tauri
cargo check
```

## 7) Troubleshooting rápido

- Se `npm` falhar no PowerShell por execution policy, corre via `cmd`:

```bash
cmd /c npm run tauri dev
```

- Se Tauri falhar por toolchain, confirma:
  - `rustup show`
  - Build Tools C++ instalados
  - WebView2 runtime instalado

## 8) Estado atual de stack

A implementação mantém o requisito pedido:

- Backend: **Tauri (Rust)**
- Frontend: **React + CodeMirror 6 (MirrorCode 6)**
- Sem Django/FastAPI/serviços extra.
