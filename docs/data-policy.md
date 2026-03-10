# Data Policy

## Objetivo

Garantir portabilidade física simples: ao mover/copiar a pasta root do projeto, todos os dados relevantes da app movem junto.

## Regra principal

Todos os dados locais persistidos pela aplicação devem ficar na pasta root selecionada pelo utilizador, dentro de:

- `.projectl-data/app_state.json`
- `.projectl-data/olm_state.json`

## Visibilidade

- `.projectl-data` deve ser ocultada no filetree da app.
- O utilizador continua a ver apenas conteúdo de estudo na árvore principal.

## Tipos de dados em `app_state.json`

- `appOnboardingDone`
- `appOpenDays`
- `lastOpenedByDomain.v1`
- `appSettings`
- `nodeLearningProfiles.v1`
- `learningAnalytics.v1`
- `olmConfigByDomain.v1`
- `appStudyPlans`

## Tipo de dados em `olm_state.json`

- Snapshot completo do OLM (conceitos, edges, estado, evidências, config, etc.).

## Compatibilidade

- O backend ainda suporta fallback de path para AppData quando nenhum path é fornecido em `olm_save_state`/`olm_load_state`.
- O frontend deve sempre preferir path explícito em `.projectl-data/olm_state.json`.
