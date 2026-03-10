# OLM Logic Map (Bridge)

Este ficheiro em `docs/` e apenas uma ponte para evitar duplicacao e drift.

## Fonte canónica por tema

- Arquitetura, riscos, limites e prioridades de implementacao:
- `docs/architecture-deep-dive.md`

- Formalizacao completa do OLM (formulas, regras de update, comandos, simulacao):
- `OLM_LOGIC_MAP.md` (na raiz do repositorio)

## Regra de manutencao

Quando houver mudancas no motor OLM:

1. Atualizar primeiro `OLM_LOGIC_MAP.md` (raiz) para formulas/regras.
2. Atualizar `docs/architecture-deep-dive.md` para impacto arquitetural e plano de implementacao.
3. Nao expandir este ficheiro com conteudo tecnico detalhado.
