# 9. Erros e exceções

;;;Exceções em Python:Funções em Python,Condicionais em Python;;;

Uma exceção interrompe o fluxo normal quando ocorre um problema. `try` permite tratar falhas que são esperadas.

```python
try:
    idade = int(input("Idade: "))
except ValueError:
    print("Escreve um número inteiro.")
else:
    print(f"Idade registada: {idade}")
finally:
    print("Operação terminada.")
```

- `except` trata um tipo específico de erro.
- `else` corre apenas se não existir exceção.
- `finally` corre sempre.
- `raise` cria deliberadamente uma exceção.

```python
def definir_percentagem(valor):
    if not 0 <= valor <= 100:
        raise ValueError("A percentagem deve estar entre 0 e 100")
    return valor
```

> Evita `except:` sem indicar o tipo: pode esconder erros de programação que deveriam ser corrigidos.

## Experimenta

Cria uma divisão segura que trate `ValueError` nas entradas e `ZeroDivisionError` no divisor.

**Pré-requisitos:** [[07-funcoes]] e [[04-condicionais]]. **A seguir:** [[10-ficheiros]].
