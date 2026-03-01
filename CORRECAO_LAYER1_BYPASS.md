# Correção Adicional: Bypass do Layer 1 para Queries com Anexos

## Problema Descoberto no Teste Real

Após a primeira correção, testamos com o comando:
```bash
zeroclaw agent -m "Pegue o arquivo hello.txt do Dropbox e envie para jvvbsj12@gmail.com"
```

### O que aconteceu:
1. ✅ Arquivo encontrado no Dropbox
2. ✅ Arquivo baixado (`DROPBOX_READ_FILE` retornou `s3key`, `mimetype`, `name`)
3. ❌ Email enviado **SEM anexo** - apenas texto no corpo

### Causa Raiz (Segunda Iteração)

Analisando os logs, descobrimos:

```
INFO zeroclaw::tools::composio_nl: Layer 1: Quick extraction successful
  arguments={"body":"...","recipient_email":"jvvbsj12@gmail.com","subject":"hello.txt"}
```

O **Layer 1 (quick extraction)** estava retornando sucesso, então o **Layer 2 (LLM)** nunca foi chamado!

#### Por que isso é um problema?

- **Layer 1**: Usa regex/pattern matching simples
  - Extrai apenas: `recipient_email`, `subject`, `body`
  - **NÃO tem acesso ao contexto da conversa**
  - **NÃO pode ver o resultado do `DROPBOX_READ_FILE`**
  - **NÃO pode incluir o campo `attachment`**

- **Layer 2**: Usa LLM com contexto completo
  - Tem acesso ao histórico da conversa
  - Pode ver o `s3key` retornado pelo `DROPBOX_READ_FILE`
  - Pode incluir o campo `attachment` com os metadados corretos

## Solução Implementada

Modificamos o Layer 1 para **detectar keywords de arquivo/anexo** e retornar `None`, forçando o uso do Layer 2.

### Keywords Detectadas

```rust
let attachment_keywords = [
    "file", "arquivo", "attach", "anexo", "anexar",
    "dropbox", "drive", "document", "documento",
    "send file", "enviar arquivo", "with file", "com arquivo"
];
```

Se a query contém qualquer uma dessas palavras, o Layer 1 retorna `None` e o Layer 2 é chamado.

### Código Adicionado

```rust
// CRITICAL: Skip Layer 1 if query mentions file/attachment keywords
let query_lower = query.to_lowercase();
let attachment_keywords = [
    "file", "arquivo", "attach", "anexo", "anexar",
    "dropbox", "drive", "document", "documento",
    "send file", "enviar arquivo", "with file", "com arquivo"
];

if attachment_keywords.iter().any(|kw| query_lower.contains(kw)) {
    tracing::debug!(
        query = query,
        "Query mentions file/attachment keywords - skipping Layer 1, will use Layer 2 (LLM)"
    );
    return None;
}
```

## Fluxo Corrigido

### Antes (Incorreto):
```
Query: "Envie arquivo do Dropbox para email"
  ↓
Layer 1: Quick extraction
  ↓ (retorna sucesso)
Resultado: {"recipient_email": "...", "subject": "...", "body": "..."}
  ↓
Email enviado SEM anexo ❌
```

### Depois (Correto):
```
Query: "Envie arquivo do Dropbox para email"
  ↓
Layer 1: Detecta keyword "arquivo"
  ↓ (retorna None)
Layer 2: LLM com contexto completo
  ↓ (vê s3key do DROPBOX_READ_FILE)
Resultado: {
  "recipient_email": "...",
  "subject": "...",
  "body": "...",
  "attachment": {
    "name": "hello.txt",
    "mimetype": "text/plain",
    "s3key": "268883/..."
  }
}
  ↓
Email enviado COM anexo ✅
```

## Como Testar Agora

### 1. Recompilar
```bash
cargo build --release
```

### 2. Testar com Logs
```bash
$env:RUST_LOG="zeroclaw=debug"
zeroclaw agent -m "Pegue o arquivo hello.txt do Dropbox e envie para jvvbsj12@gmail.com"
```

### 3. Verificar nos Logs

Procure por estas linhas:

```
DEBUG zeroclaw::tools::composio_nl: Query mentions file/attachment keywords - skipping Layer 1, will use Layer 2 (LLM)
```

E depois:

```
INFO zeroclaw::tools::composio_nl: Layer 2: LLM extraction successful
  arguments={"recipient_email":"...","attachment":{...}}
```

Se você ver ambas as linhas, a correção está funcionando!

## Casos de Teste

### Deve usar Layer 2 (LLM):
- ✅ "Envie arquivo do Dropbox para email"
- ✅ "Send file from Dropbox to email"
- ✅ "Anexe documento do drive no email"
- ✅ "Attach document from drive to email"
- ✅ "Pegue o arquivo hello.txt e envie"

### Pode usar Layer 1 (Quick):
- ✅ "Envie email para user@example.com com assunto 'Olá'"
- ✅ "Send email to user@example.com with subject 'Hello'"
- ✅ "Mande mensagem para fulano dizendo que está tudo bem"

## Limitações

### Keywords em Outros Idiomas

Atualmente suportamos:
- Português: arquivo, anexo, anexar, documento
- Inglês: file, attach, document

Se você usar outro idioma, pode ser necessário adicionar mais keywords.

### False Positives

Se a query mencionar "file" ou "arquivo" mas NÃO for sobre anexos, o Layer 2 será usado desnecessariamente. Isso não causa problemas, apenas é um pouco mais lento.

Exemplo:
- "Envie email dizendo que o file server está offline"
  - Detecta "file" → usa Layer 2
  - Layer 2 não encontra anexo → envia email normal
  - Resultado: funciona, mas poderia ter usado Layer 1

## Próximos Passos

1. **Testar em produção** com casos reais
2. **Monitorar logs** para verificar se Layer 2 está sendo usado corretamente
3. **Adicionar keywords** para outros idiomas se necessário
4. **Considerar ML** para detecção mais inteligente de intenção de anexo

## Commits Relacionados

1. `53b662df` - Correção inicial: instruções do LLM para anexos
2. `004eba27` - Esta correção: bypass do Layer 1 para queries com anexos

## Referências

- `src/tools/composio_nl.rs` - Função `try_quick_extraction`
- `COMPOSIO_ATTACHMENT_FIX.md` - Documentação da correção inicial
- `GUIA_ANEXOS_EMAIL.md` - Guia do usuário
