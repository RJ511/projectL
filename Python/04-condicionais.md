# 4. Decisões com condicionais

;;;Condicionais em Python:Operadores e interação em Python;;;

As instruções `if`, `elif` e `else` escolhem que bloco executar. A condição é avaliada como verdadeira ou falsa.

```python
nota = float(input("Nota: "))

if nota >= 18:
    print("Excelente")
elif nota >= 10:
    print("Aprovado")
else:
    print("Ainda não aprovado")
```

É possível combinar condições:

```python
idade = 20
tem_bilhete = True

if idade >= 18 and tem_bilhete:
    print("Entrada permitida")
```

Valores como `0`, `""`, `None` e coleções vazias são considerados falsos em contexto booleano.

```python
nome = ""
if not nome:
    print("Falta indicar o nome")
```

## Experimenta

Lê uma temperatura e mostra `frio`, `ameno` ou `quente`, definindo tu os limites.

**Pré-requisitos:** [[03-operadores-entrada-saida]]. **A seguir:** [[05-colecoes]].
