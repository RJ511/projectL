# 3. Operadores, entrada e saída

;;;Operadores e interação em Python:Variáveis e tipos básicos em Python;;;

Os operadores produzem novos valores a partir de outros valores.

| Grupo | Operadores | Exemplo |
|---|---|---|
| Aritmética | `+ - * / // % **` | `7 // 2` resulta em `3` |
| Comparação | `== != < <= > >=` | `idade >= 18` |
| Lógica | `and or not` | `ativo and not bloqueado` |

```python
preco = 12.50
quantidade = 3
total = preco * quantidade
tem_desconto = total >= 30

print(total, tem_desconto)
```

## Interagir com o utilizador

`input()` devolve sempre uma string. Converte-a antes de fazer cálculos numéricos. Uma *f-string* insere expressões entre chavetas.

```python
nome = input("Nome: ")
idade = int(input("Idade: "))
print(f"Olá, {nome}. No próximo ano terás {idade + 1} anos.")
```

> `=` atribui um valor; `==` compara dois valores.

## Experimenta

Pede dois números ao utilizador e mostra a soma, o produto e se o primeiro é maior do que o segundo.

**Pré-requisito:** [[02-variaveis-e-tipos]]. **A seguir:** [[04-condicionais]].
