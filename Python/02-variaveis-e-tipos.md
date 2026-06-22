# 2. Variáveis e tipos básicos

;;;Variáveis e tipos básicos em Python:Execução e sintaxe de Python;;;

Uma variável associa um nome a um valor. Não é necessário declarar antecipadamente o seu tipo.

```python
nome = "Ana"          # str: texto
idade = 24            # int: número inteiro
altura = 1.68         # float: número decimal
estuda_python = True  # bool: verdadeiro ou falso
```

## Consultar e converter tipos

`type()` indica o tipo de um valor. Funções como `int()`, `float()` e `str()` tentam convertê-lo.

```python
ano_texto = "2026"
ano = int(ano_texto)
mensagem = "Ano: " + str(ano)

print(type(ano))
print(mensagem)
```

O valor especial `None` representa a ausência intencional de um valor.

```python
resultado = None
```

## Nomes legíveis

- Usa `snake_case`: `preco_total`.
- Começa por uma letra ou `_`, nunca por um número.
- Evita nomes reservados como `if`, `for` ou `class`.

## Experimenta

Cria variáveis para um produto, o seu preço e a sua disponibilidade. Mostra numa frase os três valores.

**Pré-requisito:** [[01-primeiros-passos]]. **A seguir:** [[03-operadores-entrada-saida]].
