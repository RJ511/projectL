# 11. Classes e objetos

;;;Classes e objetos em Python:Funções em Python,Coleções em Python;;;

Uma classe define dados e comportamentos relacionados. Cada objeto é uma instância dessa classe.

```python
class Tarefa:
    def __init__(self, titulo):
        self.titulo = titulo
        self.concluida = False

    def concluir(self):
        self.concluida = True

    def resumo(self):
        estado = "✓" if self.concluida else "○"
        return f"{estado} {self.titulo}"


tarefa = Tarefa("Estudar Python")
tarefa.concluir()
print(tarefa.resumo())
```

- `__init__` inicializa o objeto.
- `self` refere-se à instância atual.
- Os atributos guardam estado; os métodos definem comportamento.

As classes são úteis quando vários valores e operações formam uma entidade clara. Para dados muito simples, um dicionário pode ser suficiente.

## Experimenta

Cria uma classe `Produto` com `nome` e `preco`, e um método que devolva o preço depois de aplicar uma percentagem de desconto.

## Fecho do percurso

Se consegues alterar os exemplos e explicar o resultado, já tens base para criar pequenos programas. Regressa ao [[00-indice]] e revê os tópicos que ainda não consegues explicar sem ajuda.

**Pré-requisitos:** [[07-funcoes]] e [[05-colecoes]].
