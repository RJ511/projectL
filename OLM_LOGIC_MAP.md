# OLM Logic Map (Project L)

Este documento descreve **a lógica completa do OLM atual**: fluxo, fórmulas, decisões, "porquê" das recomendações, sugestões e factos importantes.

## 1) Objetivo do OLM

O OLM (Open Learner Model) aqui implementado serve para:

1. Integrar eventos de estudo em formato único.
2. Manter um estado explícito por conceito (`alpha`, `beta`, `mastery`, `uncertainty`).
3. Gerar decisão "o que estudar a seguir" (`next_to_study`) com justificação.
4. Expor evidência rastreável (explicabilidade) por conceito.

## 2) Mapa de arquitetura (alto nível)

```mermaid
flowchart TD
  A[Eventos de estudo] --> B[Ingestão OLM]
  B --> C[Mapeamento para conceitos]
  C --> D[Score do evento s in 0..1]
  D --> E[Peso efetivo w]
  E --> F[Update Beta-like por conceito]
  F --> G[Estado: alpha beta mastery uncertainty]
  F --> H[Rasto de evidência]
  G --> I[Serviço de decisão next_to_study]
  H --> I
  I --> J[Top recomendações + why]
```

## 3) Modelo de dados (persistente desde v2)

O estado OLM é persistido em ficheiro JSON dentro do root do projeto (`.projectl-data/olm_state.json`) entre reinícios da app via `olm_save_state` / `olm_load_state`.

- `concepts`: conceitos
- `edges`: pré-requisito -> alvo
- `content_items`: itens de conteúdo
- `content_concepts`: mapeamento conteúdo -> conceito com `coverage_weight`
- `concept_state`: `alpha`, `beta`, `last_update`
- `evidence`: lista de `EvidenceChunk` por conceito
- **`config`**: parâmetros OLM persistidos (ver secção 15)

### Estado inicial por conceito

Sempre que um conceito aparece pela primeira vez:

- $\alpha = 1$
- $\beta = 1$

Isto evita divisões por zero e começa num estado neutro.

## 4) Fórmulas nucleares

## 4.1 Mastery e Uncertainty

Para cada conceito:

- $m = \dfrac{\alpha}{\alpha + \beta}$
- $u = \dfrac{1}{\alpha + \beta}$

Interpretação:

- `mastery` sobe com evidência positiva.
- `uncertainty` desce quando aumenta a evidência total.

## 4.2 Peso base por tipo de evento

Implementado em `event_type_weight`:

- `quiz_attempt` = 1.0
- `practice_attempt` = 0.7
- `flashcard_review` = 0.6
- `study_read` = 0.2
- `review` = 0.12
- `note_taking` = 0.08
- `self_assessment` = 0.3
- default = 0.2

## 4.3 Score do evento `s ∈ [0,1]`

Implementado em `event_score`.

### a) `quiz_attempt` e `practice_attempt`

- $s = clamp(\dfrac{correct}{total}, 0, 1)$

### b) `study_read`

- $s = clamp(\dfrac{duration\_sec}{target\_duration\_sec}, 0, 1) \times 0.5$

Leitura é explicitamente limitada para ter impacto mais fraco.

### c) `review`

- $s = clamp(\dfrac{duration\_sec}{target\_duration\_sec}, 0, 1) \times 0.35$

`review` é um sinal leve para revisão rápida.

### d) `note_taking`

- $$s = clamp(\dfrac{chars\_written}{target\_chars}, 0, 1) \times 0.35$$

`note_taking` mede produção de notas com impacto deliberadamente baixo.

### e) `flashcard_review`

- se `rating == again` -> $s = 0$
- caso contrário -> $s = 1$
- sem rating -> $s = 0.5$

### f) `self_assessment`

- usa `payload.score` ou `payload.confidence`, com `clamp`.
- fallback: $s = 0.5$

## 4.4 Peso de confiança

Implementado em `confidence_weight`:

- $w_{conf} = 0.5 + 0.5 \cdot confidence$
- Se não houver confidence -> $w_{conf}=1.0$

## 4.5 Peso efetivo total

Para cada conceito afetado:

Nota de leitura: esta secção define o pipeline completo de peso (`w`).
A secção 12 detalha apenas a componente metacognitiva (`w_meta`) usada aqui.

- peso base:

$$
w_{base}=w_{type} \cdot w_{mapping} \cdot w_{conf} \cdot w_{meta}
$$

- limitador de warmup (depende do número de evidências prévias do conceito):

$$
w_{warm}=clamp\left(\frac{n+1}{warmup\_steps(event\_type)},\ 0.15,\ 1.0\right)
$$

- hard cap por tipo de evento:

$$
w = min\left(w_{base}\cdot w_{warm},\ hard\_cap(event\_type)\right)
$$

Aqui, `w` é o peso final aplicado no update.

Onde:

- $w_{type}$: peso por tipo de evento
- $w_{mapping}$: cobertura do conteúdo para esse conceito
- $w_{conf}$: peso de confiança
- $w_{meta}$: fator metacognitivo

Definição de $w_{meta}$: ver secção **12.3**.

Caps atuais (`limiter_for_event`):

- `quiz_attempt`: 0.35
- `practice_attempt`: 0.25
- `flashcard_review`: 0.20
- `self_assessment`: 0.18
- `review` / `study_read` / `note_taking`: 0.10
- default: 0.15

## 4.6 Regra de update do estado

Cada update gera um `EvidenceChunk` com:

- `event_id`
- $\Delta \alpha = w \cdot s$
- $\Delta \beta = w \cdot (1 - s)$

- $\alpha \leftarrow \alpha + \Delta\alpha$
- $\beta \leftarrow \beta + \Delta\beta$
- `delta_alpha`, `delta_beta`
- `event_type`
- `applied_weight`
- `created_at`

- `score`
  Evidência por conceito é limitada aos últimos 200 registos.

## 5) Mapeamento evento -> conceito

A função `normalize_maps` usa esta ordem:

1. Se `event.concept_ids` existir -> usa esses IDs com peso 1.0.
2. Senão, se existir `content_id` -> resolve por `content_concepts`.
3. Sem nenhuma das duas opções -> erro de ingestão.

Isto garante que todo evento aplicado é atribuível a conceito(s).

Nota de robustez (v2):

- Pesos de mapeamento são normalizados por item/evento (somam 1.0 após deduplicação por conceito).
- `coverage_weight` inválido (não finito ou <= 0) é descartado na normalização e rejeitado no comando de mapeamento.

## 6) Lógica de recomendação (`next_to_study`)

### 6.0 Como pré-requisitos entram no grafo

No fluxo atual do editor, pré-requisitos são inferidos a partir de marcação inline no conteúdo:

- `;;;conceito;;;` -> cria/atualiza conceito inline
- `;;;conceito:prereq;;;` -> cria/atualiza `conceito`, cria/atualiza `prereq` (se não existir) e adiciona aresta `prereq -> conceito`
- `;;;conceito:pr1,pr2;;;` -> adiciona múltiplas arestas (`pr1 -> conceito`, `pr2 -> conceito`)

Todos os IDs são normalizados para o namespace do domínio no formato `<domínio>.inline.<slug>`.

Nota operacional:

- `content_concepts` e eventos automáticos de `review` usam apenas os conceitos explicitamente declarados no lado esquerdo da marcação (`conceito`), não os pré-requisitos inferidos.

## 6.1 Readiness por pré-requisito

Para cada conceito alvo `c`:

- Se não tem pré-requisitos: $R(c)=1$
- Se tem pré-requisitos $p$: usa o pior caso

$$
R(c)=\min_{p \in prereqs(c)}\big(clamp(m_p-\gamma u_p,0,1)\big)
$$

Com `gamma` (simulações atuais: `0.35`) para reduzir o efeito de incerteza excessivamente agressivo.

## 6.2 Filtro de prontidão

- Existe um corte mínimo patológico: $R(c) < R_{min}$ (simulações: `0.1`) é excluído.
- O limiar `theta` passa a funcionar como referência de “hard ready” para diagnóstico.
- A ordenação pode incluir conceitos abaixo de `theta` via soft-gating.

## 6.3 Score final

Com `lambda` default 0.7:

$$
Score(c)=R(c)^k\cdot\Big(\lambda(1-m_c)+(1-\lambda)u_c\Big)
$$

Quando não há soft-gating (`k` ausente), o comportamento volta ao hard-gate clássico.

Intuição:

- privilegia lacuna de domínio (`1 - mastery`)
- também considera incerteza (`u`)
- penaliza conceitos não prontos via `readiness`

## 6.4 Why (explicação da recomendação)

Cada item recomendado inclui 3 linhas automáticas:

1. Mastery baixo/moderado.
2. Readiness dos pré-requisitos.
3. Estado da incerteza (alta vs controlada).

No ranking unificado por item (`olm_next_content_to_study`), cada item também expõe:

- `mapping_quality` (0..1)
- `mapping_penalty` (multiplicador aplicado ao score)

Estes campos são mostrados no Domain Level para tornar a penalização visível ao utilizador.

## 6.5 Filtro de domínio no ranking por item

`olm_next_content_to_study` usa `domain_id` explícito em `content_items` para segmentar recomendações por domínio.

- Evita dependência exclusiva de prefixo de path.
- Mantém compatibilidade com `domain_filter` no ranking por conceito (`next_to_study`).

## 7) Porque este desenho é bom para OLM

1. **Explícito**: estado por conceito é observável (`alpha`, `beta`, `m`, `u`).
2. **Explicável**: toda mudança é rastreada no `evidence`.
3. **Controlável**: pesos e thresholds são regras claras, não caixa-preta.
4. **Extensível**: fácil adicionar novos tipos de evento, decay temporal e persistência.

## 8) Relação com os 3 níveis (root / domain / item)

Atualmente no projeto:

- **Root level**: visão agregada (streak de dias de abertura + frase motivacional no painel de níveis).
- **Domain level**: domínio ativo inferido por pasta de primeiro nível e recomendação principal.
- **Item level**: item selecionado e proxy simples de dificuldade por tamanho de conteúdo.

Importante: o OLM do backend hoje é **conceitual global** (não particionado por domínio de forma nativa).
A segmentação por domínio pode ser reforçada com:

1. Convenção de IDs (ex: `math.algebra.linear`).
2. Mapeamento `content_id -> concept_id` por domínio.
3. Filtros de recomendação por prefixo de domínio.

## 9) Sugestões concretas de evolução

1. **Persistência**: guardar estado OLM em ficheiro/DB.
2. **Domain-aware ranking**: endpoint para `next_to_study` por domínio.
3. **Item metrics reais**: acertos/erros por ficheiro em vez de proxy por tamanho. ✅ Implementado via `olm_get_content_metrics`.
4. **Decay temporal**: reduzir ligeiramente evidência antiga. ✅ Implementado no cálculo de estado efetivo quando `decay_enabled=true`.
5. **Qualidade de mapping**: aumentar uso de `content_concepts` para precisão. ✅ Eventos automáticos do editor agora registam `content_id`, criam `content_item` e mapeiam 1..N conceitos inline do conteúdo.

## 10) Factos e limites atuais

- Estado OLM é **persistido** em `.projectl-data/olm_state.json` via `olm_save_state` / `olm_load_state` (atomic write com rename).
- Eventos automáticos editor -> OLM estão ativos com `review` ao abrir ficheiro (`openFile` em `useFileSystem.jsx`).
- Guardar ficheiros de teste/quiz (`quiz-*` / `teste-*`) pode disparar check-in metacognitivo 1-4 e ingestão `self_assessment`.
- Eventos automáticos do editor só ingerem conceito(s) quando existem marcações inline no ficheiro (não promovem o ficheiro a conceito por defeito).
- O conteúdo pode declarar conceitos inline com `;;;nome do conceito;;;` e pré-requisitos com `;;;conceito:prereq;;;` (múltiplos: `;;;conceito:pr1,pr2;;;`), mapeados para IDs `<domínio>.inline.<slug>`.
- O catálogo canónico de conceitos inline fica persistido em `.projectl-data/app_state.json` na chave `inlineConceptCatalog.v1`, no formato `concept_id -> { name, requires[] }`.
- Fórmulas já implementadas com defaults robustos (fallbacks para score, confidence e readiness).
- Existe limite de 200 evidências por conceito para controlar crescimento.
- `stop_mastery` e `exclude_concepts` permitem mitigar root-bias e excluir conceitos ruidosos.
- `domain_filter` em `next_to_study` permite recomendações por domínio (prefixo de concept_id).
- Uncertainty formula configurável: `standard` (1/N) ou `sqrt` (1/√N).
- Decay temporal é aplicado ao estado efetivo (`alpha`/`beta`) usando semi-vida (`decay_half_life_days`) quando ativado.

## 11) Endpoints/comandos do OLM (Tauri)

### Existentes

- `olm_upsert_concept`
- `olm_list_concepts`
- `olm_add_edge`
- `olm_list_edges`
- `olm_upsert_content_item`
- `olm_map_content_concept`
- `olm_ingest_event`
- `olm_get_state`
- `olm_get_explain`
- `olm_next_to_study` (agora com `domain_filter` e `exclude` opcionais)
- `olm_next_content_to_study` (recomendação única de item baseada em conceitos priorizados)
- `olm_get_config` / `olm_set_config` — ler/gravar parâmetros OLM
- `olm_save_state` / `olm_load_state` / `olm_reset_state` — persistência do estado
- `olm_export_json` / `olm_import_json` — exportar/importar JSON completo
- `olm_get_debug_ranking` — ranking completo com decomposição de score (debug)
- `olm_get_content_metrics` — métricas por conteúdo (`attempts`, `success_rate`, `avg_score`)

> Nota: as simulações sintéticas foram movidas para **CLI de teste** e não fazem parte da superfície final da app.

## 12) Metacognição

O OLM agora usa dados metacognitivos na ingestão real (`olm_ingest_event`) via um fator adicional:

$$
w_{pre\_meta}=w_{type}\cdot w_{mapping}\cdot w_{conf}\cdot w_{meta}
$$

Este termo é o peso **antes** dos limitadores (warmup e hard-cap) e corresponde ao mesmo bloco base descrito em **4.5**.

O peso efetivamente aplicado no update continua (exatamente como em **4.5**):

$$
w=\min\left(w_{pre\_meta}\cdot w_{warm},\ hard\_cap(event\_type)\right)
$$

Com isso, o update deixa de ser apenas "resultado bruto" e passa a considerar qualidade metacognitiva sem violar os limites de segurança do modelo.

### 12.0 Porquê (decisão de produto)

O objetivo desta camada é reduzir "cegueira" do modelo quando só existe telemetria comportamental. Dois utilizadores podem ter métricas semelhantes de interação, mas estados cognitivos muito diferentes. O check-in curto 1-4 acrescenta um sinal subjetivo mínimo para:

1. Distinguir esforço produtivo vs friccao improdutiva.
2. Capturar percecao de aprendizagem entre eventos objetivos.
3. Ajustar `w_meta` sem depender apenas de acerto/erro.
4. Melhorar priorizacao quando existe ambiguidade entre candidatos no ranking.

Princípio: usar um input simples e frequente, de baixo custo cognitivo, para enriquecer a qualidade da evidência.

### 12.1 Sinais metacognitivos usados

- `confidence`
- `perceived_score` (quando existir)
- `difficulty` / `perceived_difficulty`
- `effort` / `effort_level`
- `objective_score` (derivado de `correct/total` em eventos objetivos)

Definições operacionais (runtime em `metacognitive_signal`):

- `objective_score`:
  - se o evento tiver `correct` e `total`, então:
    $$
    objective=clamp\left(\frac{correct}{total},0,1\right)
    $$
  - caso contrário: `objective` ausente.
- `perceived`:
  - prioridade 1: `payload.perceived_score` (clamped)
  - fallback implícito em alguns ramos: `confidence`.
- `difficulty`:
  - prioridade: `difficulty`, senão `perceived_difficulty`, senão `0.5`.
- `effort`:
  - prioridade: `effort`, senão `effort_level`, senão `0.6`.

Todos estes sinais são normalizados para `[0,1]` via `clamp_01`.

### 12.2 Alinhamento metacognitivo

Exemplo principal:

$$
alignment = 1 - |objective - perceived|
$$

Implementação explícita (ordem de fallback):

1. Se existir `objective` e `perceived_score`:

$$
alignment = clamp\left(1 - |objective - perceived\_score|, 0, 1\right)
$$

1. Se existir `objective` mas não `perceived_score`:

$$
alignment = clamp\left(1 - |objective - confidence|, 0, 1\right)
$$

1. Se não existir `objective`, mas existir `perceived_score`:

$$
alignment = clamp\left(1 - |perceived\_score - confidence|, 0, 1\right)
$$

1. Sem sinais suficientes: `alignment = 0.5`.

### 12.3 Fator metacognitivo

O fator final combina dificuldade, esforço, confiança e alinhamento, com intensidade controlada por `meta_strength`:
Componentes intermediárias:

$$
difficulty\_term = 1 - 0.3\cdot\left(2\cdot|difficulty-0.5|\right)
$$

$$
effort\_term = 0.7 + 0.3\cdot effort
$$

$$
confidence\_term = 0.75 + 0.25\cdot confidence
$$

$$
alignment\_term = 0.6 + 0.4\cdot alignment
$$

$$
meta\_raw = clamp\left(difficulty\_term\cdot effort\_term\cdot confidence\_term\cdot alignment\_term, 0, 1\right)
$$

$$
w_{meta}= (1-meta\_strength) + meta\_strength\cdot meta\_raw
$$

Ou seja, `meta_raw` é o score metacognitivo composto (pré-intensidade), e `meta_strength` controla quanto esse score influencia `w_meta`.

No update real atual (`olm_ingest_event`), o backend usa `meta_strength = 0.6` (balanced) como constante.

### 12.5 Mitigação de viés metacognitivo

Para reduzir sobre-ajuste por autoavaliações subjetivas, o runtime aplica quatro salvaguardas:

1. **Fiabilidade metacognitiva** `r in [0,1]`:

- combina presença de sinal objetivo, confiança, alinhamento, histórico de evidência e cap por sessão;
- quanto menor `r`, menor o impacto metacognitivo.

1. **Shrink para neutro em baixa fiabilidade**:

$$
s' = 0.5 + (s - 0.5) \cdot (0.4 + 0.6r)
$$

onde `s` é o score bruto do evento e `s'` o score efetivo usado no update.

1. **Blending com sinal objetivo (quando existir)**:

- para eventos com evidência objetiva (`correct/total`), o score final puxa para o objetivo;
- isto limita divergência quando percepção e desempenho real entram em conflito.

1. **Cap por sessão para `self_assessment`**:

- após várias autoavaliações no mesmo dia para o mesmo conceito, o efeito metacognitivo é reduzido;
- evita inflação de influência por repetição de prompts.

Implementação de referência: `src-tauri/src/commands/olm.rs` em `metacognitive_signal`, `meta_reliability`, `meta_session_cap` e `ingest_into_store`.

### 12.4 Check-ins metacognitivos 1-4 (frontend)

Além dos sinais já presentes no payload, o frontend dispara perguntas curtas com resposta inteira `1..4` (1=muito mau/difícil, 4=muito bom/fácil) em três gatilhos:

1. Após janela temporal de sessão (check-in periódico).
2. No fim de fluxo de teste/quiz (ao guardar ficheiro `quiz-*`/`teste-*`).
3. Ao sair de um domínio e entrar noutro (transição de contexto).

Cada resposta gera um evento `self_assessment` com `payload.score` normalizado para `[0,1]`, `payload.meta_reflection_rating` e `payload.meta_trigger`, influenciando o update por `w_meta`.

## 13) Cenários sintéticos automáticos (implementado)

Foi adicionado um runner **CLI** (`src-tauri/src/bin/olm_sim.rs`) para correr automaticamente cenários imaginários sob múltiplos approaches.

Execução:

- `cd src-tauri`
- `cargo run --bin olm_sim`
- `cargo run --bin olm_sim -- --json` (output JSON)
- `cargo run --bin olm_sim -- --save` (guarda em `results.json`)
- `cargo run --bin olm_sim -- --out results.json` (guarda no ficheiro indicado)
- `cargo run --bin olm_sim -- --profile quick --seed-range 0 19` (arranque rápido com diversidade)
- `cargo run --bin olm_sim -- --profile stress --scenario-mode generated --seeds 50` (somente cenários gerados)

Parâmetros úteis para gerar muitos casos distintos rapidamente:

- `--profile quick|balanced|stress` (presets de volume/densidade)
- `--generated-scenarios N` (quantidade adicional de cenários sintéticos)
- `--graph-size N`, `--event-count N`, `--prereq-density X`, `--depth N`, `--mapping-quality X`
- `--scenario-mode mixed|canonical|generated` (misto, apenas cenários canónicos, ou apenas gerados)

### 13.1 Approaches atuais

1. `baseline` — sem metacognição (`meta_strength=0`)
2. `metacog_balanced` — metacognição moderada (`meta_strength=0.6`) + **soft gating** (`k=2.0`)
3. `metacog_strict` — metacognição forte + gating mais exigente (`meta_strength=1.0`) + **soft gating** (`k=3.0`)

Todos usam `readiness_gamma=0.35` e `min_readiness=0.1` no simulador atual.

### 13.2 Cenários S1–S6

- S1 Progressão linear
- S2 Pré-requisito bloqueado
- S3 Erros repetidos no core
- S4 Incerteza alta
- S5 Multi-fonte
- S6 Cobertura parcial

### 13.3 Output do simulador

Por approach, devolve:

- `pass_rate` (Hit@1)
- `hit_at_3_rate`, `avg_mrr`, `avg_ndcg_at_3`
- métricas treino/teste (`train_pass_rate`, `test_pass_rate`, `train_avg_mrr`, `test_avg_mrr`)
- resultados por cenário com ranking completo (`top_recommendations`, `rank_of_first_expected`)
- diagnóstico por cenário:
  - `candidates_before_gate` / `candidates_after_gate` (hard-ready)
  - `candidates_ranked` / `candidates_excluded_min_readiness`
  - `readiness_distribution`
  - `concept_event_counts`
  - `top_candidates` com score/readiness/mastery/uncertainty
- `best_approach_id` no relatório final
- bloco `calibration`:
  - em cenários canónicos, seleção no treino S1–S3 e leitura no teste S4–S6;
  - em cenários gerados (`G*`), split automático 70/30 (treino/teste) por ordem de cenário;
  - métrica de seleção: `train_pass_rate` (Hit@1) -> `train_avg_mrr` -> `hit_at_3_rate`.

---

Se quiseres, no próximo passo posso gerar também uma versão "Mestrado" deste mapa, já em formato de capítulo (Problema, Formalização, Algoritmo, Avaliação, Limitações e Trabalho Futuro).

## 14) CLI de simulação — flags disponíveis (v2)

```bash
cd src-tauri

# Simulação simples (determinística)
cargo run --bin olm_sim

# Output JSON
cargo run --bin olm_sim -- --json
cargo run --bin olm_sim -- --save           # guarda em results.json
cargo run --bin olm_sim -- --out path.json  # guarda em ficheiro indicado

# Multi-seed (mostra variância entre N seeds)
cargo run --bin olm_sim -- --seeds 10
cargo run --bin olm_sim -- --seed-range 0 29 --out multi_results.json

# Calibração automatizada de meta_strength + relatório versionado
cd ..
npm run calibrate:meta
```

Multi-seed agrega métricas (pass_rate, hit@3, MRR, nDCG@3) por approach sobre N execuções.  
As execuções usam seed para perturbar ligeiramente os estados dos cenários (de forma reproduzível por seed/cenário), permitindo variância real em multi-seed.

O pipeline `calibrate:meta` grava automaticamente três artefactos em `docs/reports/`:

- `meta-strength-calibration-YYYY-MM-DD.raw.json`
- `meta-strength-calibration-YYYY-MM-DD.json`
- `meta-strength-calibration-YYYY-MM-DD.md`

Estes ficheiros servem de base para recomendar default de `meta_strength` em ambientes de teste/sintéticos e auditar estabilidade entre seeds.

## 15) Config OLM (parâmetros persistidos)

| Campo                  | Default      | Descrição                                                                                    |
| ---------------------- | ------------ | -------------------------------------------------------------------------------------------- |
| `lambda`               | 0.70         | Peso de mastery vs uncertainty no score                                                      |
| `gamma`                | 0.35         | Intensidade de penalização por incerteza na readiness                                        |
| `theta`                | 0.50         | Limiar de readiness para hard-gate                                                           |
| `min_readiness`        | 0.10         | Readiness mínima para aparecer no ranking                                                    |
| `soft_gate_k`          | 1.60         | Expoente do soft-gating (maior = mais restritivo)                                            |
| `meta_strength`        | 0.60         | Parâmetro persistido de metacognição (atualmente não aplicado no `olm_ingest_event` runtime) |
| `root_penalty`         | 0.12         | Penalização de score para conceitos raiz (sem prereqs)                                       |
| `stop_mastery`         | 0.85         | Mastery acima do qual conceitos raiz são excluídos do ranking                                |
| `exclude_concepts`     | `[]`         | Lista de concept_ids excluídos do `next_to_study`                                            |
| `uncertainty_formula`  | `"standard"` | `"standard"` (1/N) ou `"sqrt"` (1/√N)                                                        |
| `decay_enabled`        | `false`      | Ativar decay temporal no cálculo de estado efetivo                                           |
| `decay_half_life_days` | 30           | Semi-vida do decay em dias                                                                   |

### Como interpretar stop_mastery

`stop_mastery` protege contra o "root-bias": quando um conceito raiz (sem pré-requisitos) já foi bem dominado (mastery > stop_mastery), é removido das recomendações para dar lugar a conceitos mais avançados.

### Como usar exclude_concepts

Adicionar concept IDs técnicos/auxiliares ao `exclude_concepts` evita que poluam as recomendações. Continuam visíveis no `olm_get_state` e `olm_get_explain`.

## 16) Persistência — como funciona

1. `olm_save_state(path?)` — serializa o estado completo (incluindo config) para JSON, usando atomic write (escreve `.tmp` e rename).
2. `olm_load_state(path?)` — carrega o JSON e substitui o estado em memória.
3. `olm_reset_state()` — repõe o estado ao default (vazio).
4. `olm_export_json()` — devolve o JSON como string (para debug/backup manual).
5. `olm_import_json(json)` — importa JSON de string.

Na política atual da app, o frontend passa path explícito para gravar/carregar em `.projectl-data/olm_state.json` no root selecionado. Sem `path`, o backend continua a suportar fallback em AppData.

## 17) Debug Ranking

`olm_get_debug_ranking(top?, domain_filter?, exclude?)` devolve:

- `candidates`: todos os candidatos rankeados com campos adicionais:
  - `gate_factor`: fator do soft-gating (1.0 se ready, < 1.0 se não)
  - `event_count`: número de evidências já registadas para o conceito
  - `why`: lista de razões textuais para o score
- `diagnostics`: `candidates_before_gate`, `candidates_after_gate`, `candidates_ranked`, `candidates_excluded_min_readiness`, `readiness_distribution`, `concept_event_counts`, `top_candidates`

Visível no OLM Panel → "Debug Ranking" no frontend.

## 18) Limitações e próximos passos

1. **Item-aware ranking**: existe recomendação única de item (`olm_next_content_to_study`) orientada por conceitos; a qualidade depende de cobertura/qualidade de `content_concepts`.
2. **Domain-aware automático**: atualmente o `domain_filter` é passado explicitamente pelo frontend; uma inferência automática pela pasta ativa pode ser integrada em `Main.jsx`.
3. **Testes de integração end-to-end**: smoke tests manuais documentados são o próximo passo (ver PROJECT_GUIDE.md).
