# 1. Primeiros passos

;;;Execução e sintaxe de Python;;;

Python executa instruções pela ordem em que aparecem. Um ficheiro Python usa normalmente a extensão `.py`.

```python
print("Olá, Python!")
print(2 + 3)
```

Guarda o exemplo como `ola.py` e executa-o num terminal:

```bash
python ola.py
```

## Regras essenciais

- `#` inicia um comentário.
- A indentação, habitualmente com **quatro espaços**, define blocos de código.
- Python distingue maiúsculas de minúsculas: `nome` e `Nome` são identificadores diferentes.
- Uma instrução termina no fim da linha; normalmente não se usa `;`.

```python
# Isto é um comentário
if 5 > 2:
    print("A indentação coloca esta linha dentro do if")
```

> Um `IndentationError` significa geralmente que os espaços de um bloco não estão alinhados.

## Experimenta

- [ ] Mostra o teu nome com `print()`.
- [ ] Acrescenta um comentário que explique a instrução.
- [ ] Provoca e depois corrige um erro de indentação.

**A seguir:** [[02-variaveis-e-tipos]].
