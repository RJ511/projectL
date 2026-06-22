# 6. Ciclos

;;;Ciclos em Python:Condicionais em Python,Coleções em Python;;;

Um ciclo `for` percorre os elementos de uma sequência. `range()` gera uma sequência de números.

```python
nomes = ["Ana", "Bruno", "Carla"]

for nome in nomes:
    print(f"Olá, {nome}")

for numero in range(1, 4):
    print(numero)  # 1, 2, 3
```

Um ciclo `while` repete enquanto a condição for verdadeira.

```python
contador = 3
while contador > 0:
    print(contador)
    contador -= 1
```

- `break` termina o ciclo.
- `continue` salta para a iteração seguinte.
- `enumerate()` fornece o índice e o valor.

```python
for indice, nome in enumerate(nomes, start=1):
    print(indice, nome)
```

> Num `while`, garante que algo altera a condição; caso contrário, crias um ciclo infinito.

## Experimenta

Percorre uma lista de números e mostra apenas os números pares, juntamente com a sua soma.

**Pré-requisitos:** [[04-condicionais]] e [[05-colecoes]]. **A seguir:** [[07-funcoes]].
