This repository follows GitHub's spec-kit workflows.

- For every command, locate the matching prompt in `.github/prompts/*.prompt.md` and execute the instructions exactly as written.
- The library of commands is defined by the files present in `.github/prompts/`; review that directory to understand the supported surface area.
- If a command arrives without a corresponding prompt file, stop and flag it as unknown rather than improvising a response.
- When a prompt is found, adhere to it precisely (variable substitutions, ordering, formatting, etc.).
- There are no additional constraints beyond what each prompt specifies.

Example: for `/specify these are the arguments`, use `.github/prompts/specify.prompt.md` with `$ARGUMENTS="these are the arguments"` and follow the template verbatim.
