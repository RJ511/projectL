# 8. Módulos e importações

;;;Módulos em Python:Funções em Python;;;

Um módulo é um ficheiro `.py` que contém código reutilizável. A biblioteca padrão já fornece muitos módulos.

```python
import math
from random import randint

print(math.sqrt(81))
print(randint(1, 6))
```

Também podes importar um módulo teu. Se `calculos.py` contiver uma função `dobro`, outro ficheiro na mesma pasta pode fazer:

```python
from calculos import dobro

print(dobro(5))
```

O bloco seguinte só é executado quando o ficheiro é iniciado diretamente:

```python
def main():
    print("Programa iniciado")


if __name__ == "__main__":
    main()
```

Pacotes de terceiros são normalmente instalados com `python -m pip install nome_do_pacote`. Confirma sempre a documentação do pacote antes de o instalar.

## Experimenta

Cria `conversoes.py` com uma função que converta Celsius para Fahrenheit e importa-a noutro ficheiro.

**Pré-requisito:** [[07-funcoes]]. **A seguir:** [[09-erros-e-excecoes]].
