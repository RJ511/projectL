# Architecture Deep Dive (Canonical)

Este e o documento tecnico canonico para arquitetura, logica OLM e decisoes de implementacao.

## 1. Scope e fonte de verdade

- Documento canonico: `docs/architecture-deep-dive.md`.
- Formula detalhada OLM: `OLM_LOGIC_MAP.md` (raiz) como referencia matematica.
- Guia operacional: `docs/PROJECT_GUIDE.md`.
- Ideacao historica: `docs/bainstorm.md` (nao usar como verdade tecnica).

## 2. Arquitetura end-to-end

- Frontend: React/Vite, orquestrado em `src/pages/Main.jsx`.
- Backend: Tauri/Rust, comandos em `src-tauri/src/commands/*`.
- Persistencia local no root selecionado:
- `.projectl-data/app_state.json` para estado de app.
- `.projectl-data/olm_state.json` para snapshot OLM.
- Integracoes opcionais: AnkiConnect (`http://127.0.0.1:8765`).

## 3. Responsabilidades por modulo

### 3.1 Frontend

- `src/pages/Main.jsx`
- Orquestra niveis `root`/`domain`/`item`.
- Controla paineis OLM, Planner e Settings.
- Faz load/save de estado OLM no ciclo de vida da app.

- `src/hooks/useFileSystem.jsx`
- Estado operacional de arvore/ficheiro/editor.
- Persistencia de analytics e perfis via `rootDataStore`.
- Ingestao automatica no OLM baseada em conceitos inline (`;;;conceito;;;`) e prerequisitos inline (`;;;conceito:prereq;;;`).
- Check-ins metacognitivos 1..4 por tempo, fim de teste e transicao de dominio.

- `src/components/insights/KnowledgeLevels.jsx`
- Visualizacao de dominio/root.
- Recomendacao de item unificada via `nextContentToStudy`.

- `src/components/olm/OlmPanel.jsx`
- Operacoes manuais OLM (ingestao, explain, ranking, debug).
- Visualizacao interativa de grafo de conceitos/arestas com filtro textual, arestas direcionais e selecao de conceito para abrir evidencias + ficheiros associados.
- Largura do painel ajustavel por drag na borda/canto direito para melhor encaixe no layout.

- `src/components/settings/AppSettingsPanel.jsx`
- Config da app + parametros OLM por dominio.

### 3.2 Backend

- `src-tauri/src/commands/fs.rs`
- CRUD de ficheiros/pastas no root selecionado.

- `src-tauri/src/commands/tree.rs`
- Projecao da arvore para UI.
- Oculta `.projectl-data` do filetree.

- `src-tauri/src/commands/olm.rs`
- Estado e atualizacao por conceito (`alpha`, `beta`, `mastery`, `uncertainty`).
- Ranking por conceito (`olm_next_to_study`).
- Ranking unificado por item (`olm_next_content_to_study`).
- Explain/debug/config/persistencia.

## 4. Fluxos criticos

### 4.1 Abrir ficheiro

1. Sidebar seleciona no.
2. `Main.handleOpenNode` define contexto de dominio.
3. `useFileSystem.openFile` le conteudo.
4. Atualiza analytics/perfis.
5. Extrai conceitos inline (`;;;conceito;;;`) e prerequisitos (`;;;conceito:prereq;;;`).
6. Faz upsert dos conceitos inline declarados e dos pré-requisitos inferidos, cria arestas prereq->target e atualiza `content_concepts` apenas para os conceitos declarados no ficheiro.
7. Envia evento `review` ao OLM apenas com os conceitos inline declarados.

### 4.2 Guardar ficheiro

1. Editor grava via `write_file`.
2. Atualiza catalogo de conceitos inline.
3. Se ficheiro for `quiz-*` ou `teste-*`, pode disparar check-in meta 1..4.

### 4.3 Recomendacao unificada

1. OLM ranqueia conceitos prioritarios.
2. OLM agrega sinal por item via `content_concepts`.
3. Frontend apresenta um ranking unico de item por dominio.

## 5. Factos e limites atuais (importante para implementar)

- OLM e estado de app ja estao centralizados no root (`.projectl-data`).
- `.projectl-data` esta oculto no filetree da app.
- O backend e concept-first; item ranking depende da qualidade de `content_concepts`.
- Ficheiro nao e promovido automaticamente a conceito; o sinal conceitual vem de marcacoes inline no conteudo.
- Eventos automaticos existem, mas a cobertura de eventos objetivos ainda e parcial.
- Check-in metacognitivo 1..4 adiciona sinal subjetivo util, mas suscetivel a ruido.
- Segmentacao de recomendacao por item agora usa `domain_id` explicito em `content_items`.
- A incerteza por conceito usa formula de variancia Beta (`alpha*beta / ((alpha+beta)^2*(alpha+beta+1))`) no runtime OLM.
- O peso de ingestao foi organizado em tres blocos equivalentes no runtime (`w_evidence`, `r_reliability`, `g_safety`) sem alterar o peso final aplicado.
- O peso de confianca do evento usa distancia a objetivo: `w_conf = clamp(1 - confidence_mismatch_lambda*|confidence-objective|, 0, 1)` com fallback `1.0` quando faltam sinais.
- A ingestao valida primeiro os conceitos resolvidos: eventos sem qualquer conceito existente sao rejeitados antes de entrar em `study_events`; eventos repetidos por `event_id` devolvem no-op com `duplicate=true`.
- A criacao de arestas (`olm_add_edge`) rejeita ciclos no grafo de prerequisitos, alem de self-edges e IDs inexistentes.
- A readiness usa ancestrais multi-hop no grafo com blend controlado por `readiness_blend_eta`: `R(c) = eta*R_min + (1-eta)*R_mean`, onde `R_min = min_{a in Ancestors(c)}(delta^d(a,c) * r_a)` e `R_mean` e a media desses mesmos termos, com `r_a = clamp(m_a - gamma*u_a, 0, 1)`.
- O ranking e orientado por politica (`ranking_policy`): `learn_next` (lacuna de mastery), `review_next` (incerteza + decay + inconsistencia), `adaptive` (switch por `theta`) e `balanced` (formula classica com `ranking_lambda`).
- Modelo de estado com decay in-place: `concept_state.alpha/beta` guardam estado efetivo em `last_update`; antes de ler ou atualizar um conceito, o backend aplica decay de `last_update -> now`, e no update persiste `last_update = now`.

## 6. Riscos tecnicos e pontos de atencao (importante para resolver)

- Drift de documentacao entre varios ficheiros.
- Mitigacao implementada: ficheiro canonico + regra obrigatoria em `.github/copilot-instructions.md` + script `npm run check:docs-sync`.

- Qualidade de mapeamento item-conceito insuficiente.
- Impacto: recomendacao de item perde precisao.
- Mitigacao implementada: penalizacao por qualidade de mapeamento no backend e visibilidade de `mapping_penalty`/`mapping_quality` na UI de dominio.

- Dependencia de convencoes de prefixo para dominio.
- Impacto: erros silenciosos de filtro/ranking.
- Mitigacao implementada (fluxo de item-ranking): filtro por `domain_id` explicito em `olm_next_content_to_study`; normalizacao/namespacing de perfis no frontend e validacao estrita de `domain_id` em `olm_upsert_content_item`.

- Corrupcao de estado local (`app_state.json`).
- Impacto: perda de contexto/plano/config.
- Mitigacao implementada: escrita resiliente com `temp + rename`, backup (`app_state.bak.json`) e recuperacao automatica no load.

- Sinal metacognitivo enviesado por fadiga/contexto.
- Impacto: `w_meta` pode sobre-ajustar.
- Mitigacao implementada (Fase 2): fiabilidade metacognitiva, shrink para neutro com baixa fiabilidade, cap por sessao para `self_assessment` e blending com evidencia objetiva.

## 7. Sugestoes concretas de evolucao (priorizadas)

1. Fortalecer mapeamento `content_concepts` (obrigatorio para melhorar ranking unico).
2. Adicionar validacoes de dominio/ID na camada de settings e ingestao.
3. Implementar escrita atomica para `app_state.json` com rollback simples.
4. Cobrir fluxos criticos com testes automatizados, com backend unit tests ja presentes para:

- abrir ficheiro -> evento,
- guardar quiz/teste -> meta check-in,
- trocar dominio -> meta check-in,
- ranking unico por dominio.

1. Medir calibracao de `meta_strength` com cenarios sinteticos e logs reais anonimizados.

2. Automatizar pipeline de calibracao e relatorio versionado em `docs/reports/`.

Status atual:

- Itens 1, 2 e 3 implementados; o item 4 tem cobertura backend automatizada para os fluxos criticos do OLM, mas nao existe ainda uma suite e2e dedicada no workspace.
- Item 6 implementado: script `scripts/calibrate-meta-strength.mjs` + `npm run calibrate:meta`.
- Item 5 parcialmente implementado: calibracao sintetica pronta; falta integrar logs reais anonimizados na segunda fase de validacao.
- A calibracao no simulador foi ajustada para priorizar `Hit@1` (`train_pass_rate`) e usar split adaptativo em cenarios gerados (70/30).

## 8. Decisoes de implementacao imediata

Quando houver mudancas no motor/fluxos, atualizar obrigatoriamente:

1. `docs/architecture-deep-dive.md` (canonico).
2. `OLM_LOGIC_MAP.md` (raiz, se formulas/regras mudarem).
3. `docs/PROJECT_GUIDE.md` (se fluxo operacional mudar).

## 9. Referencias de codigo

- Frontend: `src/pages/Main.jsx`, `src/hooks/useFileSystem.jsx`, `src/components/insights/KnowledgeLevels.jsx`, `src/components/olm/OlmPanel.jsx`, `src/components/settings/AppSettingsPanel.jsx`.
- Services: `src/services/olm.service.jsx`, `src/services/rootDataStore.js`, `src/services/dataPolicy.js`, `src/services/fs.service.js`, `src/services/tauriBridge.js`.
- Backend: `src-tauri/src/main.rs`, `src-tauri/src/commands/fs.rs`, `src-tauri/src/commands/tree.rs`, `src-tauri/src/commands/olm.rs`.
