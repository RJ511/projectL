Este projeto refere-se a uma aplicação da qual possui o objetivo primário de auxiliar qualquer individuo, focalizado em autodidatas. Devido a não ter um campo em específico do conhecimento a aplicação é extremamente dependente da boa utilização do utilizador para conseguir colher os melhores frutos.

## Tipos de ficheiros

    - Root. Pasta onde todo o projeto estará situado.
    - Subjects. Pastas com caminho no Root. Cada Subject terá o seu oml próprio exposto para o user.
    - OML.
    - Teste. Ficheiros que serem como avaliações. Devem ter uma estrutura padronizada. Fácil desenvolvimento para qualquer um.
    - Decks. Alguma maneira de utilizar decks do anki e servir para alimentar os dados internamente.
    - Objetivos, Tarefas, Dias, temporização etc
    - Padrão. Ficheiros padrão são a base do programa, ficheiros .md que não têm um propósito em especifico para além de qualquer qualquer tipo de informação.

https://www.leiderschapsdomeinen.nl/wp-content/uploads/2016/12/Zimmerman-B.-2002-Becoming-Self-Regulated-Learner.pdf
Aprendizagem: - Antes (Definir objetivos) - Durante - Apoós (Auto-reflexão)

## Definir objetivos

Definir objetivos é uma das partes mais importantes do processo de aprendização auto-regulada, fazendo com que esta parte seja de extrema importância e devido à grande quantidade de possibildiades é impossível desenvolver algo à medida para cada tópico.
Tendo isto em conta, e seguindo com a visão de que o user deve saber melhor do que ninguém o que pretende fazer, o nosso objetivo é apenas auxiliar. **A maioria dos objetivos terão que ser preenchidos pelo próprio user** deixando o resto nas mãos das templates que serão apresentadas, com o objetivo apenas de dar uma ideia MUITO generalizada desta mecânica.

### Templates

(pesquisar sobre os diferentes campos do conhecimento para encontrar padrões que possam ser utilizados para desenvolver estas templates)
"1. Compreensão

“Compreender / identificar / conhecer / definir / distinguir”

2. Produção

“Criar / escrever / sintetizar / produzir / formular”

3. Aplicação

“Aplicar / resolver / exemplificar / demonstrar”

4. Revisão

“Rever / atualizar / reestruturar / corrigir / testar”

5. Expansão

“Explorar / aprofundar / procurar / investigar”

6. Reflexão

“Refletir / avaliar / analisar / melhorar”
"

### Tipos

#### Objetivos Definitivos

    Objetivos que teoricamente não alteram, são criados num momento e seguem até ao fim
    Exemplo:
    - -[ ] Aprender Hiragana (sem sinais)
    - -[ ] Aprender Hiragana (com sinais)
    - -[ ] Aprender 10 Kanjis

#### Objetivos Ocasionais

    Objetivos ocasionais são tarefas que aparecem espontaneamente como reação a algo
    Exemplo:
    - Não foi revisto x conteudo em y tempo, Objetivo diário -[ ] Rever conteudo x
    - Ver um vídeo sobre x

## OML

### Tópico

    - Titulo
    - Descrição
    - Ultima vez (?BD?)
    - Dificuldade
    - Acertos
    - Consistência

#### Consistência

    Consistência, neste sistema, não é definida por dias seguidos de estudo (streak), uma métrica errónea e injusta.
    Em vez disso, consistência representa o grau em que o utilizador cumpre o seu ritmo próprio de estudo — definido por ele ou inferido automaticamente pelo sistema.
    Assim, o OLM calcula consistência comparando o intervalo real entre sessões e o intervalo esperado, gerando uma medida contínua entre 0 e 1 que reflete regularidade sustentável e autenticidade do processo de aprendizagem.
    Isto torna o sistema inclusivo, personalizado e alinhado com evidência sobre autorregulação da aprendizagem.

# Origem dos dados

---

Revisão de Literatura:

A literatura em metacognição descreve a aprendizagem como um processo que envolve controlo e monitorização. Nelson e Narens propõe um modelo de dois níveis - object-level e meta-level - no qual o meta-level consiste na representação dinâmica do estado cognitivo subjacente. Neste modelo, a monitorização corresponde ao fluxo de informação do object-level para o meta-level, enquanto o controlo representa as ações que afetam o object-level com base nessa informação.

Modelos de aprendizagem autoregulada alinham-se com esta distinção, que caracteriza a aprendizagem como um processo cíclico. Panadero (2017) sintetiza diversos modelos de SRL que descrevem fases iterativas de antecipação, executação e autoreflexão, nas quais a informação resultante da monitorização informa decisões subsquentes ao longo do ciclo.

Dado que a monitorização requer a exposição de informação sobre o estado do conhecimento, torna-se necessário caracterizar como essa informação é representada em sistemas computacionais. Bull & Kay propõem a framework SMILI para descrever e analisar decisões de design relacionadas com a viabilidade do modelo. Esta framework descreve níveis como a forma de visualização, detalhe, grau de abertura e interação permitida.

A literatura em learning analytics evidencia que a simples apresentação de métricas desconexas não é o suficiente para suportodar processos de decisão. Jivet et al. (2018) demonstra que dashboards eficazes dependem de um alinhamento claro entre o modelo , nivel de abstração da informação apresentada e o tipo de decisões que se pretende apoiar. Isto sugere que a visibilidade do estado de conhecimento deve prioritizar representações interpretáveis e acionáveis, evitando o exceço de informação e carga cognitiva.

A visibilidade do estado de conhecimento, quando operacionalizada através de dashboards derivados de modelos explícitos, pode suportar processos de monitorização e decisão, desde que a informação seja selecionada, agregada e alinhada com o tipo de decisão a apoiar.

---

Resumo importante literatura:

Nelson e Narens (2002)
Nelson e Narens dividem a metacognição em 3 principios abstractos, onde o primeiro identifica que os processos cognitivos são divididos em 2 níveis que apesar de distintos estão correlacionados, object-level e meta-level. Meta-level consiste em um nivel dinamico, neste caso de estudo, uma simulação mental do object-level, segundo principio. E entre estes níveis, o terceiro principio, relações dominantes que dependem da direção entre níveis. Monitorização, de object-meta e controlo de meta-object.

Panadero (2017)

Panadero informa que SRL é um processo ciclico dividido em 3 fases: a previsão ou antecipação, a performace e auto-reflexão. Onde consecutivamente, o individo define os seus objetivos e planeia todas as estratégias correspondentes, seguido da sua implementação, onde executão ações de controlo através do processo de aprendizagem e auto-regulação sobre auto-eficácia, motivação e expectativa. Por fim, auto-reflexão, é utilzado toda a informação e experiências obtidas para voltar ao inicio do ciclo.

Bull and Kay (2016)

Bull e Kay apresentam uma framework de descrição de OLM, onde aborda niveis e exposição, granulidade, detalhe, interação.

---

0. O que estás a fazer (em 1 frase)

Estás a construir um motor de Open Learner Model (OLM) para aprendizagem autodidata (fora de LMS) que integra eventos de estudo, mantém um estado explícito + explicável do conhecimento e gera uma decisão: “o que estudar a seguir”.

1. Artefacto (o protótipo) — módulos
   M1 — Modelação de domínio (grafo de conceitos)

CRUD de conceitos/skills e relações de pré-requisito.

Import/export (JSON/YAML) para poderes criar domínios rápido.

M2 — Registo de conteúdo + mapeamento conteúdo→conceitos

“ContentItem” (nota/deck/avaliação/vídeo/pdf) e respetivo mapeamento para conceitos.

Manual (obrigatório). Automático via LLM (opcional, “extra”, não core).

Proposta_Mestrado_IPSantarem_v3…

M3 — Event pipeline (ingestão)

Um schema único de eventos (“o que aconteceu”) para integrar ferramentas num PLE (o teu contexto).

M4 — OLM Engine (core)

Atualiza o estado por conceito a partir dos eventos.

Mantém incerteza e rasto de evidência (explicabilidade).

M5 — Decision service: “Next to study”

Devolve 3–10 recomendações com justificação (“porquê este e não outro?”).

Regras explícitas (não ML opaco).

M6 — UI mínima (visibilidade)

Lista/árvore de conceitos com mastery+incerteza.

Vista de detalhe com “porque” + evidências + recomendação.

Isto está alinhado com o que escreves: integrar modelação de domínio, estados e mecanismos de exposição; formalizar OLM; protótipo; suportar decisão.

2. Definição formal do OLM (forma simples, robusta e explicável)
   2.1 Domínio

Um grafo dirigido: G = (V, E)

V = conceitos

E = (pré-requisito → dependente)

2.2 Estado por conceito

Para cada conceito
c∈V
c∈V, o estado do aprendiz é:

S(c)=⟨αc, βc, tc, evidencec⟩
S(c)=⟨α
c
​

, β
c
​

, t
c
​

, evidence
c
​

⟩

Onde:

α,β
α,β são parâmetros de uma Beta (evidência pró vs contra)

mastery
mc=αcαc+βc
m
c
​

=
α
c
​

+β
c
​

α
c
​

    ​

uncertainty
uc=1αc+βc
u
c
​

=
α
c
​

+β
c
​

1
​

(quanto menor a evidência total, maior a incerteza)

tc
t
c
​

último update

evidence = lista resumida de contribuições (para explicar)

Isto dá-te: estado explícito + incerteza + explicabilidade, sem “caixa-preta”.

3. Eventos (schema) — o centro da integração
   3.1 Tipos mínimos (chega para a tese)

study_read (leitura/consumo)

practice_attempt (exercício/questão)

quiz_attempt (avaliação agregada)

flashcard_review (SRS)

self_assessment (auto-rating)

3.2 Estrutura padrão (JSON)
{
"event_id": "uuid",
"timestamp": "2026-02-23T21:10:00Z",
"source": "anki|notion|manual|simulator",
"event_type": "practice_attempt",
"content_id": "uuid-or-null",
"concept_ids": ["uuid1", "uuid2"],
"payload": {
"duration_sec": 180,
"correct": 1,
"total": 1,
"confidence": 0.7,
"difficulty": 0.5
}
}
3.3 Regra: evento pode mapear para N conceitos

Ou vem concept_ids já preenchido

Ou vem content_id e resolves via ContentConceptMap

4. Algoritmo de update (OLM Engine) — regra clara e defensável
   4.1 Pesos por tipo de evento (exemplo)

quiz_attempt: peso 1.0 (evidência forte)

practice_attempt: peso 0.7

flashcard_review: peso 0.6

study_read: peso 0.2 (evidência fraca)

self_assessment: peso 0.3 (útil mas subjetivo)

4.2 Atualização por conceito

Para cada conceito
c
c afetado pelo evento:

Calcula um score
s∈[0,1]
s∈[0,1]:

para tentativas:
s=correcttotal
s=
total
correct
​

para leitura:
s=min⁡(1, durationtarget)
s=min(1,
target
duration
​

) \* 0.5 (cap fraco)

para flashcards:
s=1
s=1 se “good/easy”,
0
0 se “again”

Peso efetivo
w=wtype⋅wmapping⋅wconfidence
w=w
type
​

⋅w
mapping
​

⋅w
confidence
​

w_mapping: se o conteúdo cobre muito o conceito, 1.0; se parcial, 0.5

w_confidence: opcional, ex.
0.5+0.5⋅confidence
0.5+0.5⋅confidence

Atualiza Beta:

αc←αc+w⋅s
α
c
​

←α
c
​

+w⋅s

βc←βc+w⋅(1−s)
β
c
​

←β
c
​

+w⋅(1−s)

(Opcional) decadência temporal (para revisão futura, sem virar foco):

aplica um decaimento leve em
α,β
α,β se passaram muitos dias sem evidência.

4.3 Guardar explicação (rasto)

Em cada update guardas um “chunk”:

event_id

contribuição para α/β

regra aplicada (tipo, peso, score)

Isto suporta “inspecionável”, que é central em OLM e na tua ênfase de “visibilidade do estado”.

5. Decisão A: “O que estudar a seguir” (NextToStudy)
   5.1 Ideia

Escolher conceitos aprendíveis agora (pré-requisitos OK) e com maior ganho esperado (mastery baixo ou incerteza alta).

5.2 Cálculos

Define:

mc
m
c
​

mastery

uc
u
c
​

uncertainty

P(c)
P(c) pré-requisitos diretos

readiness do conceito:

R(c)=min⁡p∈P(c)(mp⋅(1−up))
R(c)=
p∈P(c)
min
​

(m
p
​

⋅(1−u
p
​

))

Se não tem pré-requisitos:
R(c)=1
R(c)=1

Score final para recomendação:

Score(c)=R(c)⋅(λ(1−mc)+(1−λ)uc)
Score(c)=R(c)⋅(λ(1−m
c
​

)+(1−λ)u
c
​

)

λ
λ ~ 0.7 (prioriza lacuna, mas considera incerteza)

Filtro:

só recomendas se
R(c)≥θ
R(c)≥θ (ex. 0.6)

5.3 Output (com explicação)

Para cada recomendação devolves:

concept_id, score, mastery, uncertainty, readiness

why: 3 bullets automáticos:

“Mastery baixo (X)”

“Pré-requisitos OK (min=Y)”

“Evidência recente insuficiente” ou “Erros recentes”, etc.

Isto liga diretamente ao teu problema técnico (“estado explícito integrado que suporte decisões de estudo”) e ao objetivo geral.

6. Modelo de dados (PostgreSQL) — mínimo mas completo
   Tabelas

concepts(id, domain_id, name, description, metadata_json)

concept_edges(id, prereq_concept_id, target_concept_id, weight)

content_items(id, type, title, source, metadata_json)

content_concepts(id, content_id, concept_id, coverage_weight)

events(id, timestamp, source, type, content_id, payload_json)

event_concepts(id, event_id, concept_id, mapping_weight)

olm_state(concept_id, alpha, beta, last_update)

olm_evidence(id, concept_id, event_id, delta_alpha, delta_beta, rule_json, created_at)

Notas:

olm_evidence pode ser “top N últimos 200 registos” para não crescer infinito.

7. API (Django + DRF ou FastAPI) — endpoints do MVP
   Ingestão

POST /events (recebe evento, resolve conceitos, aplica updates)

POST /simulate/events (gera e injeta eventos sintéticos)

Domínio

GET/POST /concepts

GET/POST /concept-edges

GET/POST /content-items

POST /content-items/{id}/map-concepts

OLM / Visibilidade

GET /olm/state (lista conceitos com mastery+uncertainty)

GET /olm/state/{concept_id}

GET /olm/explain/{concept_id} (rasto: evidências e contribuições)

Decisão

GET /decisions/next-to-study?top=10

8. UI mínima (React) — só o necessário

3 ecrãs:

Mapa/Lista de conceitos

mastery (barra) + uncertainty (ícone/valor)

Detalhe do conceito

“porque”: eventos que mais contribuíram

Next To Study

top 10 recomendações + 3 bullets de justificação

SMILI entra aqui apenas como grelha de decisão de design (o teu relatório usa para justificar “como exponho o estado”, não para virar projeto de UI gigante).

9. Demonstração + avaliação (sem humanos, com dados sintéticos)

Isto está alinhado com “natureza conceptual e computacional”, “dados sintéticos”, e avaliação das dificuldades técnicas/conceptuais.

9.1 Cenários sintéticos (obrigatório)

Cria 6 scripts de simulação:

S1 Progressão linear: eventos corretos → mastery sobe; recomendações avançam no grafo.

S2 Pré-requisitos bloqueiam: mastery baixo no prereq → recomendações ficam no prereq.

S3 Erros repetidos: aumenta β → recomenda voltar ao conceito/ao prereq.

S4 Incerteza alta: poucos eventos → recomenda “avaliar/praticar” esse conceito.

S5 Multi-fonte: eventos de duas “tools” → estado integra igual.

S6 Cobertura parcial: content cobre 2 conceitos com pesos → updates diferentes.

9.2 Testes (critério de suficiência)

Invariantes:
0<m<1
0<m<1,
u
u diminui quando há mais evidência.

Monotonicidade controlada: sequência de acertos aumenta
m
m em média.

Gating: se prereq < θ, alvo não aparece no top.

Explicabilidade: toda recomendação tem evidências/razões.

9.3 “Dificuldades técnicas e conceptuais” (capítulo)

Mapping conteúdo→conceito (ruído)

Peso de tipos de evento (sensibilidade)

Decadência/recência (trade-offs)

Limites de dados sintéticos (validade externa)
Isto é literalmente um objetivo específico teu.

10. Backlog em 3 fases (para caber num mestrado)
    Fase 1 — Base (tese, obrigatório)

M1, M3, M4, M5 (next-to-study), M6 UI mínima

simulações S1–S4 + testes

Fase 2 — Extensão (se houver tempo)

content registry + mapping (M2)

simulações S5–S6

Fase 3 — Extras (trabalho futuro)

LLM mapping, conectores reais, dashboards avançados

Isto bate certo com o teu cronograma “protótipo base” + “extensão”.

Proposta_Mestrado_IPSantarem_v3…

11. Checklist: “isto já é suficiente para a tese?”

É suficiente se tiveres:

Framework descrita (módulos + fluxos + decisões suportadas)

Proposta_Mestrado_IPSantarem_v3…

OLM formal (estado + relações + update)

Protótipo funcional (ingestão → update → visibilidade → decisão)

Explicação (“porquê este estado / porquê esta recomendação”)

Demonstração com dados sintéticos + testes + discussão de limitações
