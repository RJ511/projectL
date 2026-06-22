# 10. Leitura e escrita de ficheiros

;;;Ficheiros em Python:Exceções em Python,Ciclos em Python;;;

`open()` abre um ficheiro. O bloco `with` garante que este é fechado, mesmo quando ocorre um erro.

```python
with open("notas.txt", "w", encoding="utf-8") as ficheiro:
    ficheiro.write("Primeira nota\n")
    ficheiro.write("Segunda nota\n")
```

Modos frequentes:

| Modo | Efeito |
|---|---|
| `"r"` | lê um ficheiro existente |
| `"w"` | escreve, substituindo o conteúdo |
| `"a"` | acrescenta no fim |

```python
with open("notas.txt", "r", encoding="utf-8") as ficheiro:
    for linha in ficheiro:
        print(linha.strip())
```

Para caminhos mais claros e portáveis, usa `pathlib`:

```python
from pathlib import Path

caminho = Path("dados") / "mensagem.txt"
print(caminho.exists())
```

## Experimenta

Guarda três tarefas num ficheiro, uma por linha, e depois lê-as para uma lista.

**Pré-requisitos:** [[09-erros-e-excecoes]] e [[06-ciclos]]. **A seguir:** [[11-classes-e-objetos]].
