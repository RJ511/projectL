# Project L — Implementation Context (Single Source)

Este ficheiro é a referência rápida para **implementar, alterar ou explicar** o projeto sem precisar de reconstruir contexto do zero.

## 1) Ordem de leitura (source of truth)

1. `IMPLEMENTATION_CONTEXT.md` (este ficheiro): mapa de módulos, funções, onde mudar.
2. `OLM_LOGIC_MAP.md`: formalização e lógica matemática do OLM.
3. `PROJECT_GUIDE.md`: execução e uso da app.
4. `bainstorm.md`: histórico/ideias (não usar como verdade técnica atual).

## 2) Arquitetura atual (resumo)

- Frontend: React (Vite), foco em 3 níveis (`root`, `domain`, `item`).
- Backend: Tauri + Rust, com motor OLM em `src-tauri/src/commands/olm.rs`.
- Persistência local:
  - OLM state/config: backend (`olm_save_state` / `olm_load_state`).
  - UI/context analytics: `localStorage` (hooks frontend).

## 3) Mapa de ficheiros e responsabilidades

### Entry points

- `src/pages/Main.jsx`
  - Orquestra níveis Root/Domain/Item.
  - Decide quando mostrar `EditorContainer` vs `KnowledgeLevels`.
  - Gere domínio ativo e “último ficheiro por domínio”.
  - Faz auto-load do estado OLM no arranque e auto-save periódico/fecho.

- `src-tauri/src/main.rs`
  - Registo de comandos Tauri expostos ao frontend.

### Core frontend

- `src/hooks/useFileSystem.jsx`
  - Estado da árvore, ficheiro selecionado, abrir/guardar/criar/renomear.
  - Perfis por nó (conceito/dificuldade/domínio).
  - Parsing de conceitos inline no editor via marcação `;;;conceito;;;` (compatível também com `;;conceito;;`).
  - Eventos automáticos OLM (ex.: `review` ao abrir).
  - Telemetria de uso (`learningAnalytics`): tempo por ficheiro/domínio/sessão, aberturas, etc.

- `src/components/insights/KnowledgeLevels.jsx`
  - Painel contextual para Root/Domain.
  - Recomendação de item por domínio via `getContentMetrics`.
  - Mostra uso de recursos, tempos e conclusão média.

- `src/components/olm/OlmPanel.jsx`
  - Ferramentas manuais do OLM (conceitos, arestas, evento manual, explain, next).
  - Filtragem por domínio ativo.

- `src/components/settings/AppSettingsPanel.jsx`
  - Config app + config OLM por domínio.
  - Edição manual de perfil por nó (conceito/dificuldade).

- `src/components/layout/Sidebar.jsx`
  - Navegação Root View / Domain View e seleção de domínio.

### Core backend (OLM)

- `src-tauri/src/commands/olm.rs`
  - Modelo de dados OLM (`Concept`, `StudyEvent`, `ConceptState`, etc.).
  - Ingestão de eventos e update de estado.
  - Ranking `next_to_study`, explain/debug, persistência, métricas por conteúdo.

- `src-tauri/src/commands/mod.rs`
  - Re-export dos comandos `fs`, `tree`, `olm`.

### Bridge de serviços

- `src/services/olm.service.jsx`
  - Wrapper frontend para comandos Tauri OLM.

## 4) Símbolos/funções-chave (onde mexer)

## 4.1 Níveis e navegação

- Nível atual e routing UI: `src/pages/Main.jsx`
  - `export default function Main`.
  - `currentLevel` (`item`/`root`/`domain`).
  - `handleOpenNode` (seleção de nó e domínio).

## 4.2 Perfis por domínio/sub-domínio/item

- Normalização de IDs de conceito: `src/hooks/useFileSystem.jsx`
  - `normalizeConceptId(rawConceptId, domainId)`
  - `buildDefaultProfile(node)`
  - `normalizeProfile(profile, node)`

Regra atual: `conceptId` é sempre namespaced por domínio e pode incluir sub-domínios por pontos (ex.: `mathematics.calculus.calculo-1`).

Também é possível declarar conceitos inline no conteúdo com `;;;conceito;;;`; esses conceitos são normalizados para IDs namespaced (`<domain>.inline.<slug>`) e usados no evento automático ao abrir ficheiro.

### O que influencia um concept

1. Perfil do nó (`conceptId`, `difficulty`) em `useFileSystem.jsx`.
2. Marcação inline no conteúdo (`;;;nome do conceito;;;`).
3. Eventos ingeridos (`review`, `practice_attempt`, etc.) e respetivo payload (`correct`, `total`, `confidence`, duração, etc.).
4. Mapeamento conteúdo→conceito (`content_concepts`) e cobertura (`coverage_weight`).
5. Config OLM (`meta_strength`, `gamma`, `lambda`, `soft_gate_k`, `decay_*`, etc.).

### O que o concept influencia

1. Estado OLM por conceito (`alpha`, `beta`, `mastery`, `uncertainty`).
2. Ranking `next_to_study` e justificações (`why`).
3. Explainability (`evidence` por conceito).
4. Readiness por pré-requisitos e efeito das arestas `prereq -> target`.
5. Filtro por domínio (`domain_filter`) e diagnóstico de ranking (`debug`).

## 4.3 Eventos automáticos e analytics

- Ingestão automática de evento: `trackOlmEvent(eventType, node, payload)` em `useFileSystem.jsx`.
- Abrir ficheiro (`openFile`) gera evento leve de revisão (`review`).
- Guardar ficheiro (`saveFile`) **não** gera evento de prática.
- Sessão/tempo:
  - `startActiveFileTimer`
  - `closeActiveFileTimer`
  - `closeSessionWindow`

## 4.4 Lógica OLM (backend)

- Pesos por tipo: `event_type_weight` em `olm.rs`.
- Score por evento: `event_score`.
- Mapeamento evento→conceito: `normalize_maps`.
- Limitadores fortes por evento: `limiter_for_event`.
- Update principal: `ingest_into_store`.
- Ranking: `rank_next_to_study`.

## 4.5 Recomendação no Domain Level

- `KnowledgeLevels.jsx` usa `getContentMetrics()` e filtra por `content_id` do domínio.
- O card de domínio recomenda **itens** (ficheiros), não conceitos/domínios.

## 5) Comandos OLM expostos (contrato backend)

Definidos em `olm.rs`, registados em `src-tauri/src/main.rs`:

- Conceitos/grafo: `olm_upsert_concept`, `olm_list_concepts`, `olm_add_edge`, `olm_list_edges`
- Conteúdo/mapa: `olm_upsert_content_item`, `olm_map_content_concept`
- Eventos/estado: `olm_ingest_event`, `olm_get_state`, `olm_get_explain`, `olm_next_to_study`
- Config/persistência: `olm_get_config`, `olm_set_config`, `olm_reset_state`, `olm_export_json`, `olm_import_json`, `olm_save_state`, `olm_load_state`
- Diagnóstico/métricas: `olm_get_debug_ranking`, `olm_get_content_metrics`

## 6) Lógica fora do OLM_LOGIC_MAP (importante)

Nem toda a lógica operacional está no `OLM_LOGIC_MAP.md`. Itens hoje implementados no código e relevantes:

- Limitadores de ingestão (warmup + hard cap + score suavizado): `olm.rs`.
- Regras de evento automático do editor e telemetria de uso: `useFileSystem.jsx`.
- Separação de níveis na UI (`root/domain/item`): `Main.jsx` + `KnowledgeLevels.jsx`.
- Config OLM por domínio no frontend: `AppSettingsPanel.jsx`.

## 7) “Se eu quiser mudar X, onde mudo?”

- Mudar peso/tipo de evento OLM: `event_type_weight` e `event_score` em `olm.rs`.
- Tornar subida de mestria mais/menos agressiva: `limiter_for_event` e `ingest_into_store` em `olm.rs`.
- Alterar critérios da recomendação de item no domínio: `loadSuggestion` em `KnowledgeLevels.jsx`.
- Alterar quando eventos automáticos são emitidos: `openFile` / `saveFile` / `trackOlmEvent` em `useFileSystem.jsx`.
- Alterar hierarquia Root/Domain/Item e renderização: `Main.jsx` + `Sidebar.jsx` + `KnowledgeLevels.jsx`.
- Alterar contratos frontend↔backend: `src/services/olm.service.jsx` e `src-tauri/src/main.rs`.

## 8) Estado conhecido e manutenção

- `bainstorm.md` está desatualizado e deve ser tratado como material de ideação.
- Ao introduzir lógica nova no OLM, atualizar:
  1. `OLM_LOGIC_MAP.md` (formalização)
  2. `IMPLEMENTATION_CONTEXT.md` (mapa técnico)
  3. `PROJECT_GUIDE.md` (uso/execução se afetar fluxo)

## 9) Checklist de PR/alteração

1. O comportamento mudou em Root/Domain/Item? validar `Main.jsx` + `KnowledgeLevels.jsx`.
2. OLM mudou (peso, score, ranking, comandos)? validar `olm.rs` + `OLM_LOGIC_MAP.md`.
3. Eventos automáticos mudaram? validar `useFileSystem.jsx`.
4. Build/frontend e cargo check executados.

## 10) Onde está explicado “concept”

- Visão formal do conceito e fórmulas: `OLM_LOGIC_MAP.md` (secções 1, 3, 4, 5, 6).
- Contrato técnico e pontos de alteração no código: `IMPLEMENTATION_CONTEXT.md` (secções 4.2, 4.4, 5).
- Uso operacional no UI (criação/ingestão): `PROJECT_GUIDE.md` (secções 4.3 e 5).
- Mapa completo de variáveis: `PROJECT_VARIABLES_MINDMAP.md`.
