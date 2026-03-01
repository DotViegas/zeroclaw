# Guia: Como Enviar Arquivos do Dropbox por Email

## Problema Anterior

Quando você pedia para o agente enviar um arquivo do Dropbox por email, ele:
- ✅ Encontrava o arquivo
- ✅ Baixava o arquivo
- ❌ Enviava email SEM anexo (apenas texto informativo)

## Solução Implementada

O código foi atualizado para que o LLM (modelo de linguagem) reconheça quando precisa anexar um arquivo e use os metadados corretos.

## Como Usar Agora

### Comando Natural

Você pode usar comandos naturais como:

```bash
zeroclaw agent -m "Pegue o arquivo hello.txt do Dropbox e envie para user@example.com"
```

Ou em português:

```bash
zeroclaw agent -m "Envie o arquivo relatório.pdf do meu Dropbox para chefe@empresa.com com assunto 'Relatório Mensal'"
```

### O que o Agente Faz Automaticamente

1. **Busca o arquivo** no Dropbox usando `DROPBOX_SEARCH_FILE_OR_FOLDER`
2. **Baixa o arquivo** usando `DROPBOX_READ_FILE` (obtém `s3key`, `mimetype`, `name`)
3. **Envia o email** usando `GMAIL_SEND_EMAIL` COM o campo `attachment`

### Estrutura do Anexo

O anexo é enviado no formato `FileUploadable` da Composio:

```json
{
  "recipient_email": "user@example.com",
  "subject": "Arquivo do Dropbox",
  "body": "Segue o arquivo anexo",
  "attachment": {
    "name": "hello.txt",
    "mimetype": "text/plain",
    "s3key": "268883/dynamic-module-load/READ_FILE/response/abc123"
  }
}
```

## Verificando se Funcionou

### Nos Logs

Procure por estas linhas nos logs (com `RUST_LOG=zeroclaw=debug`):

```
INFO zeroclaw::tools::composio_nl: Layer 2: LLM extraction successful
  arguments={"recipient_email":"user@example.com","subject":"...","body":"...","attachment":{...}}
```

Se você ver o campo `attachment` nos argumentos, significa que funcionou!

### No Email

O destinatário deve receber:
- ✅ Email com assunto e corpo
- ✅ Arquivo anexado (pode baixar)

## Limitações

### Tamanho do Arquivo
- Gmail: máximo ~25MB por mensagem
- Arquivos maiores falharão no envio

### Contexto do LLM
- O modelo precisa "ver" o resultado do `DROPBOX_READ_FILE` para extrair o `s3key`
- Se o contexto for muito longo, o modelo pode não incluir o anexo
- Solução: use comandos diretos e específicos

### Tipos de Arquivo Suportados
- Qualquer tipo de arquivo que o Dropbox suporte
- O `mimetype` é detectado automaticamente pelo Dropbox

## Exemplos de Comandos

### Básico
```bash
zeroclaw agent -m "Envie hello.txt do Dropbox para user@example.com"
```

### Com Assunto Personalizado
```bash
zeroclaw agent -m "Envie o arquivo contrato.pdf do Dropbox para cliente@empresa.com com assunto 'Contrato para Assinatura'"
```

### Com Corpo Personalizado
```bash
zeroclaw agent -m "Envie relatório.xlsx do Dropbox para gerente@empresa.com com assunto 'Relatório Q1' e mensagem 'Segue o relatório do primeiro trimestre para sua análise'"
```

### Múltiplos Destinatários (se suportado)
```bash
zeroclaw agent -m "Envie apresentação.pptx do Dropbox para equipe@empresa.com e chefe@empresa.com"
```

## Troubleshooting

### Email Enviado Sem Anexo

**Sintoma**: Email chega, mas sem arquivo anexado

**Possíveis Causas**:
1. Arquivo não foi baixado corretamente do Dropbox
2. LLM não incluiu o campo `attachment`
3. `s3key` expirou (improvável, mas possível)

**Solução**:
- Execute com logs detalhados: `$env:RUST_LOG="zeroclaw=debug"`
- Verifique se `DROPBOX_READ_FILE` retornou `s3key`
- Verifique se `GMAIL_SEND_EMAIL` incluiu `attachment` nos argumentos

### Erro "File Not Found"

**Sintoma**: Erro ao buscar arquivo no Dropbox

**Solução**:
- Verifique o nome exato do arquivo (case-sensitive)
- Verifique o caminho (use `/` para raiz)
- Confirme que o arquivo existe no Dropbox

### Erro "Attachment Too Large"

**Sintoma**: Erro ao enviar email

**Solução**:
- Arquivo maior que 25MB
- Use link compartilhado do Dropbox ao invés de anexo:
  ```bash
  zeroclaw agent -m "Crie um link compartilhado para arquivo_grande.zip no Dropbox e envie o link para user@example.com"
  ```

## Testando a Correção

Execute o teste automatizado:

```bash
cargo test --test composio_email_attachment
```

Todos os 5 testes devem passar:
- ✅ `attachment_structure_is_valid`
- ✅ `email_with_attachment_structure_is_valid`
- ✅ `llm_prompt_includes_attachment_instructions`
- ✅ `quick_extraction_does_not_handle_attachments`
- ✅ `workflow_download_then_send_with_attachment`

## Próximos Passos

Se você ainda tiver problemas:

1. Verifique a conexão com Dropbox: `zeroclaw channel doctor`
2. Verifique a conexão com Gmail: `zeroclaw channel doctor`
3. Teste manualmente cada etapa:
   ```bash
   # Passo 1: Buscar arquivo
   zeroclaw agent -m "Liste arquivos no meu Dropbox"
   
   # Passo 2: Baixar arquivo específico
   zeroclaw agent -m "Baixe o arquivo hello.txt do Dropbox"
   
   # Passo 3: Enviar email (agora deve incluir anexo)
   zeroclaw agent -m "Envie hello.txt para user@example.com"
   ```

## Feedback

Se encontrar problemas ou tiver sugestões, abra uma issue no GitHub com:
- Comando usado
- Logs completos (com `RUST_LOG=zeroclaw=debug`)
- Comportamento esperado vs. observado
