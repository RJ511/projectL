# 5. Coleções

;;;Coleções em Python:Variáveis e tipos básicos em Python;;;

As coleções guardam vários valores numa única estrutura.

| Tipo | Característica | Exemplo |
|---|---|---|
| `list` | ordenada e mutável | `["maçã", "pera"]` |
| `tuple` | ordenada e imutável | `(10, 20)` |
| `dict` | pares chave–valor | `{"nome": "Ana"}` |
| `set` | valores únicos | `{"azul", "verde"}` |

```python
frutas = ["maçã", "pera"]
frutas.append("laranja")
print(frutas[0])
print(len(frutas))

pessoa = {"nome": "Ana", "idade": 24}
pessoa["cidade"] = "Porto"
print(pessoa["nome"])
```

Os índices começam em `0`. Índices negativos contam a partir do fim. Um *slice* seleciona uma parte da sequência.

```python
numeros = [10, 20, 30, 40]
print(numeros[-1])  # 40
print(numeros[1:3]) # [20, 30]
```

Usa `in` para testar pertença: `"pera" in frutas`.

## Experimenta

Cria um dicionário para um livro e uma lista com três desses dicionários.

**Pré-requisito:** [[02-variaveis-e-tipos]]. **A seguir:** [[06-ciclos]].
