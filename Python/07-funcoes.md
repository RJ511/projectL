# 7. Funções

;;;Funções em Python:Ciclos em Python;;;

Uma função agrupa instruções reutilizáveis. Pode receber parâmetros e devolver um resultado com `return`.

```python
def calcular_media(valores):
    if not valores:
        return None
    return sum(valores) / len(valores)


media = calcular_media([12, 15, 18])
print(media)
```

É possível definir valores por omissão e passar argumentos pelo nome.

```python
def saudar(nome, saudacao="Olá"):
    return f"{saudacao}, {nome}!"

print(saudar("Rita"))
print(saudar(nome="Rui", saudacao="Bom dia"))
```

As variáveis criadas dentro da função são locais. Uma *docstring* explica a finalidade da função.

```python
def dobro(numero):
    """Devolve o dobro de um número."""
    return numero * 2
```

## Experimenta

Escreve uma função `maior(a, b)` que devolva o maior dos dois valores e testa-a com três pares diferentes.

**Pré-requisito:** [[06-ciclos]]. **A seguir:** [[08-modulos]].
