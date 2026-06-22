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
- **Concept Graph**: visualiza conceitos e arestas num grafo com filtro por `id` ou nome.
- **Concept Graph (interativo)**: nós clicáveis para abrir Evidence, arestas com direção (setas) e lista de ficheiros associados ao conceito selecionado.
- **Painel OLM ajustável**: arrasta com o rato na borda direita ou no canto inferior direito para personalizar a largura do painel.
- **Evento**: injeta eventos manuais (`practice_attempt`, `quiz_attempt`, etc.).
- **Estado**: mostra `mastery` e `uncertainty` por conceito.
- **Next to study**: recomendações ordenadas com a razão principal visível, prontidão, domínio e incerteza; o detalhe expansível apresenta todos os fatores usados pelo ranking e a política ativa.
- **Explicação**: rasto de evidências por conceito em linguagem humana (conteúdo, momento, sinal observado, peso, confiança e duração); `alpha`/`beta` permanecem disponíveis apenas no detalhe técnico.

Como usar o filtro do grafo:

1. No bloco **Concept Graph**, escreve parte do `id` ou nome do conceito.
2. O painel mostra os conceitos correspondentes e expande para vizinhos diretos ligados por arestas.
3. Limpa o campo para voltar a ver o grafo completo do domínio ativo.

Interação adicional:

1. Clica num nó no grafo para abrir as evidências desse conceito no bloco **Evidence**.
2. No mesmo bloco, vês os ficheiros atualmente mapeados para esse conceito.
3. Expande **Ver cálculo técnico** apenas quando precisares de auditar os incrementos Beta; a interpretação pedagógica aparece primeiro.

## 5) Eventos automáticos já integrados

Sem ação extra do utilizador:

- Ao **abrir um ficheiro**, é registado um evento `review`.
- Ao **guardar um ficheiro**, não é injetado evento automático por omissão.

Os eventos automáticos do editor usam apenas os conceitos declarados no conteúdo do ficheiro (o ficheiro não vira conceito por defeito).

Sintaxe suportada no conteúdo:

- `;;;nome do conceito;;;`
- `;;;conceito:prereq;;;`
- `;;;conceito:pr1,pr2;;;`

Esses conceitos são normalizados para IDs namespaced por domínio (`<dominio>.inline.<slug>`), e os pré-requisitos geram arestas `prereq -> conceito` no grafo OLM.

Quando usas `;;;conceito:prereq;;;`, o `prereq` também é criado/atualizado como conceito inline (se ainda não existir), para a aresta ser válida no OLM.

Check-ins metacognitivos (self-assessment):

- A app pode abrir um diálogo in-app 1..4 em três triggers: `periodic`, `test_end` e `domain_switch`.
- O diálogo é não bloqueante (sem `window.prompt`) e inclui opção de adiar (`Agora não`).
- Existe cooldown por trigger e cooldown próprio para dismiss, além de cap de prompts por sessão.

## 6) Validação rápida

### Frontend build

```bash
npm run build
```

### Verificação de sync código↔docs

```bash
npm run check:docs-sync
```

Falha se houver alterações de código sem alteração de documentação canónica.

### Calibração de `meta_strength` (sintético)

```bash
npm run calibrate:meta
```

Gera relatório versionado em `docs/reports/`:

- `meta-strength-calibration-YYYY-MM-DD.raw.json` (payload completo por seed)
- `meta-strength-calibration-YYYY-MM-DD.json` (sumário agregado)
- `meta-strength-calibration-YYYY-MM-DD.md` (recomendação legível)

Parâmetros opcionais:

- `node scripts/calibrate-meta-strength.mjs --start 0 --end 29`
- `node scripts/calibrate-meta-strength.mjs --start 10 --end 59 --out-dir docs/reports`

### Avaliação sintética final (tese)

```bash
cd src-tauri

# canónicos (S1-S12)
cargo run --bin olm_sim -- --scenario-mode canonical --json -o canonical.json

# multi-seed balanced
cargo run --bin olm_sim -- --profile balanced --seeds 50 -o balanced_multi.json

# multi-seed stress
cargo run --bin olm_sim -- --profile stress --seeds 50 -o stress_multi.json

# gerados em volume
cargo run --bin olm_sim -- --scenario-mode generated --generated-scenarios 100 --seeds 50 -o generated_multi.json

# opcional: stress mais robusto
cargo run --bin olm_sim -- --profile stress --seeds 100 -o stress_100seeds.json
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

## 9) Política de dados local (atualizada)

Todos os dados locais da app passam a ser guardados dentro da pasta root do projeto, em:

- `.projectl-data/app_state.json` (estado de UI, analytics, perfis, planos, configs de domínio)
- `.projectl-data/app_state.bak.json` (backup do estado de UI)
- `.projectl-data/olm_state.json` (estado completo do OLM)

Comportamento esperado:

- A pasta `.projectl-data` é **invisível no filetree da aplicação**.
- Ao mover/copiar fisicamente a pasta root do projeto, os dados de uso e OLM acompanham automaticamente.

Notas operacionais:

- OLM save/load é feito com path explícito para `.projectl-data/olm_state.json`.
- O estado de UI deixou de depender de `localStorage` e usa ficheiro no root.
- `app_state.json` usa escrita resiliente (`temp + rename`) com recuperação automática a partir do backup em caso de corrupção.
