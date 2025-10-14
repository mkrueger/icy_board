/**
 * PPL (PCBoard Programming Language) Tree-Sitter Grammar
 * Case-insensitive version
 *
 * Notes:
 * - Keywords, types, and builtins are now case-insensitive
 * - Identifiers accept mixed case
 * - Builtin lists aggregated from versions 1.00–4.00 (IcyBoard extensions included where known)
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

// Helper to create case-insensitive regex for keywords
function ci(keyword) {
  return new RegExp(
    keyword
      .split('')
      .map(char => {
        if (/[a-zA-Z]/.test(char)) {
          const lower = char.toLowerCase();
          const upper = char.toUpperCase();
          return `[${lower}${upper}]`;
        }
        // Handle special regex characters
        if (/[.*+?^${}()|[\]\\]/.test(char)) {
          return `\\${char}`;
        }
        return char;
      })
      .join('')
  );
}

// Helper for keyword tokens - uses alias to create a consistent token name
// Types get higher precedence (3) than other keywords (2)
function kw(keyword, precedence = 2) {
  return alias(token(prec(precedence, ci(keyword))), keyword.toUpperCase());
}

// Helper for preprocessor directives (starts with ; or ;$)
function dir(directive) {
  return token(ci(directive));
}

module.exports = grammar({
  name: "ppl",

  // Whitespace + comments
  extras: $ => [
    /\s+/,
    $.comment,
  ],

  // Conflicts - only the essential ones
  conflicts: $ => [
    [$.if_single_line_statement, $.if_block_statement],
    [$.top_level_item, $.statement],
    [$.array_dimensions, $.argument_sequence],
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => seq(
      optional($.preprocessor_section),
      repeat($.top_level_item),
    ),

    // ---------- Preprocessor ----------
    preprocessor_section: $ => repeat1($.preprocessor_directive),

    preprocessor_directive: $ => choice(
      $.define_directive,
      $.undef_directive,
      $.include_directive,
      $.if_directive,
      $.elif_directive,
      $.else_directive,
      $.endif_directive,
      $.version_directive,
    ),

    define_directive: $ => prec.right(seq(
      dir(';$DEFINE'), 
      field('name', $.identifier), 
      optional(field('value', $.expression))
    )),
    undef_directive:   $ => seq(dir(';$UNDEF'), field('name', $.identifier)),
    include_directive: $ => seq(dir(';$INCLUDE'), field('path', $.string_literal)),
    if_directive: $ => prec.right(seq(dir(';$IF'), field('cond', $.expression))),
    elif_directive: $ => prec.right(seq(dir(';$ELIF'), field('cond', $.expression))),
    else_directive:    $ => dir(';$ELSE'),
    endif_directive:   $ => dir(';$ENDIF'),
    version_directive: $ => seq(dir(';#'), field('key', $.identifier), '=', field('value', $.expression)),

    // ---------- Top level ----------
    top_level_item: $ => choice(
      $.function_declaration,
      $.function_implementation,
      $.procedure_declaration,
      $.procedure_implementation,
      $.variable_declaration,
      $.statement,
      $.label,
    ),

    // ---------- Declarations ----------
    function_declaration: $ => seq(
      kw('DECLARE'), kw('FUNCTION'),
      field('name', $.identifier),
      '(', optional($.parameter_list), ')',
      field('return_type', $.type),
    ),

    function_implementation: $ => seq(
      kw('FUNCTION'),
      field('name', $.identifier),
      '(', optional($.parameter_list), ')',
      optional(field('return_type', $.type)),
      repeat($.statement),
      kw('ENDFUNC'),
    ),

    procedure_declaration: $ => seq(
      kw('DECLARE'), kw('PROCEDURE'),
      field('name', $.identifier),
      '(', optional($.parameter_list), ')'
    ),

    procedure_implementation: $ => seq(
      kw('PROCEDURE'),
      field('name', $.identifier),
      '(', optional($.parameter_list), ')',
      repeat($.statement),
      kw('ENDPROC'),
    ),

    parameter_list: $ => seq(
      $.parameter,
      repeat(seq(',', $.parameter)),
    ),

    parameter: $ => seq(
      optional(kw('VAR')),
      field('type', $.type),
      field('name', $.identifier),
      optional($.array_dimensions),
    ),

    // ---------- Variables ----------
    // Give variable_declaration highest precedence to resolve ambiguity
    variable_declaration: $ => prec(11, prec.right(seq(
      field('type', $.type),
      field('name', $.identifier),
      optional($.array_dimensions),
      optional(seq('=', field('initializer', $.expression))),
    ))),

    array_dimensions: $ => seq(
      '(',
      $.expression,
      repeat(seq(',', $.expression)),
      ')'
    ),

    // ---------- Statements ----------
    statement: $ => choice(
      $.variable_declaration,
      $.let_statement,
      $.if_single_line_statement,
      $.if_block_statement,
      $.select_statement,
      $.while_single_line_statement,
      $.while_block_statement,
      $.repeat_until_statement,
      $.loop_block_statement,
      $.for_block_statement,
      $.goto_statement,
      $.gosub_statement,
      $.return_statement,
      $.break_statement,
      $.continue_statement,
      $.block_statement,
      $.predefined_call,
      $.procedure_call,
      $.end_statement,
      $.stop_statement,
      $.expression_statement,
    ),
    
    expression_statement: $ => $.expression,

    let_statement: $ => prec.left(1, seq(
      optional(kw('LET')),
      field('target', $.expression),
      choice('=', '+=', '-=', '*=', '/=', '%=', '&=', '|='),
      field('value', $.expression)
    )),

    if_single_line_statement: $ => seq(
      kw('IF'),
      optional('('), field('condition', $.expression), optional(')'),
      field('then', $.statement)
    ),

    if_block_statement: $ => seq(
      kw('IF'),
      optional('('), field('condition', $.expression), optional(')'),
      optional(kw('THEN')),
      repeat($.statement),
      repeat($.elseif_block),
      optional($.else_block),
      kw('ENDIF')
    ),

    elseif_block: $ => seq(
      kw('ELSEIF'),
      optional('('), field('condition', $.expression), optional(')'),
      optional(kw('THEN')),
      repeat($.statement)
    ),

    else_block: $ => seq(
      kw('ELSE'),
      repeat($.statement)
    ),

    select_statement: $ => seq(
      kw('SELECT'),
      optional(kw('CASE')),
      optional('('), field('selector', $.expression), optional(')'),
      repeat($.case_block),
      optional($.default_block),
      kw('ENDSELECT')
    ),

    case_block: $ => seq(
      kw('CASE'),
      $.case_specifier_list,
      optional(':'),
      repeat($.statement)
    ),

    case_specifier_list: $ => seq(
      $.case_specifier,
      repeat(seq(',', $.case_specifier))
    ),

    case_specifier: $ => choice(
      $.expression,
      seq($.expression, '..', $.expression)
    ),

    default_block: $ => seq(
      kw('DEFAULT'),
      optional(':'),
      repeat($.statement)
    ),

    while_single_line_statement: $ => seq(
      kw('WHILE'),
      optional('('), field('condition', $.expression), optional(')'),
      field('body', $.statement)
    ),

    while_block_statement: $ => seq(
      kw('WHILE'),
      optional('('), field('condition', $.expression), optional(')'),
      kw('DO'),
      repeat($.statement),
      kw('ENDWHILE')
    ),

    repeat_until_statement: $ => seq(
      kw('REPEAT'),
      repeat($.statement),
      kw('UNTIL'),
      optional('('), field('condition', $.expression), optional(')')
    ),

    loop_block_statement: $ => seq(
      kw('LOOP'),
      repeat($.statement),
      kw('ENDLOOP')
    ),

    for_block_statement: $ => prec.right(seq(
      kw('FOR'),
      field('var', $.identifier),
      '=',
      field('start', $.expression),
      kw('TO'),
      field('end', $.expression),
      optional(seq(kw('STEP'), field('step', $.expression))),
      repeat($.statement),
      kw('NEXT'),
      optional(field('var_repeat', $.identifier))
    )),

    goto_statement: $ => seq(kw('GOTO'), field('label', $.identifier)),
    gosub_statement: $ => seq(kw('GOSUB'), field('label', $.identifier)),

    return_statement: $ => choice(
      prec(1, seq(kw('RETURN'), field('value', $.expression))),
      kw('RETURN')
    ),

    break_statement: $ => kw('BREAK'),
    continue_statement: $ => kw('CONTINUE'),
    end_statement: $ => kw('END'),
    stop_statement: $ => kw('STOP'),

    block_statement: $ => prec(1, seq(
      kw('BEGIN'),
      repeat($.statement),
      kw('END')
    )),

    predefined_call: $ => prec.right(2, seq(
      field('name', $.builtin_statement),
      optional(choice($.parenthesized_args, $.bare_args))
    )),

    procedure_call: $ => prec.right(1, seq(
      field('name', $.identifier),
      optional(choice($.parenthesized_args, $.bare_args))
    )),

    parenthesized_args: $ => seq('(', optional($.argument_sequence), ')'),
    bare_args: $ => $.argument_sequence,

    argument_sequence: $ => seq(
      $.expression,
      repeat(seq(',', $.expression))
    ),

    // ---------- Expressions ----------
    expression: $ => choice(
      $.binary_expression,
      $.unary_expression,
      $.parens_expression,
      $.call_expression,
      $.member_reference,
      $.index_expression,
      $.identifier,
      $.constant
    ),
  
    call_expression: $ => prec(10, seq(
      field('function', choice($.identifier, $.builtin_function)),
      '(',
      optional($.argument_sequence),
      ')'
    )),

    parens_expression: $ => prec(1, seq('(', $.expression, ')')),

    member_reference: $ => prec(4, seq(
      field('object', $.expression),
      '.',
      field('member', $.identifier)
    )),

    index_expression: $ => prec(3, seq(
      field('array', $.expression),
      '(',
      field('index', $.expression),
      repeat(seq(',', field('index', $.expression))),
      ')'
    )),

    binary_expression: $ => {
      const ops = [
        ['||', 1],
        ['&&', 2],
        ['|',  3],
        ['&',  4],
        ['==', 5], ['!=', 5], ['<>', 5],
        ['<',  6], ['>',  6], ['<=', 6], ['>=', 6],
        ['+',  7], ['-',  7],
        ['*',  8], ['/',  8], ['%',  8],
        ['^',  9], ['**', 9],
      ];
      return choice(
        ...ops.map(([op, precedence]) =>
          prec.left(precedence,
            seq(
              field('left', $.expression),
              op,
              field('right', $.expression)
            )
          )
        )
      );
    },

    unary_expression: $ => prec.right(10, choice(
      seq('!', field('operand', $.expression)),
      seq(kw('NOT'), field('operand', $.expression)),
      seq('-', field('operand', $.expression)),
      seq('+', field('operand', $.expression))
    )),

    constant: $ => choice(
      $.string_literal,
      $.number_literal,
      $.boolean_literal,
      $.at_color_code
    ),

    // ---------- Labels ----------
    label: $ => seq(':', field('name', $.identifier)),

    // ---------- Comments ----------
    comment: $ => token(choice(
      /;[ \t]*[^$#\n][^\n]*/,   // generic line comment
      /'[^\n]*/,                // apostrophe comment
      /\*[ \t][^\n]*/           // star + space comment (legacy style)
    )),

    // ---------- Types ----------
    // Types get higher precedence (3) to avoid being interpreted as identifiers
    type: $ => choice(
      kw('BOOLEAN', 3), kw('DATE', 3), kw('DDATE', 3), kw('INTEGER', 3), kw('SDWORD', 3), 
      kw('LONG', 3), kw('MONEY', 3), kw('STRING', 3), kw('TIME', 3),
      kw('BIGSTR', 3), kw('EDATE', 3), kw('REAL', 3), kw('FLOAT', 3), kw('DREAL', 3), 
      kw('DOUBLE', 3), kw('UNSIGNED', 3), kw('DWORD', 3), kw('UDWORD', 3),
      kw('BYTE', 3), kw('UBYTE', 3), kw('WORD', 3), kw('UWORD', 3), kw('SBYTE', 3), 
      kw('SHORT', 3), kw('SWORD', 3), kw('INT', 3), kw('MSGAREAID', 3), kw('PASSWORD', 3)
    ),

    // ---------- Builtin Statements ----------
    builtin_statement: $ => choice(
      kw('ADJBYTES'), kw('ADJDBYTES'), kw('ADJTBYTES'), kw('ADJTFILES'), kw('APPEND'), 
      kw('BACKUP'), kw('BITSET'), kw('BITCLEAR'), kw('BYE'), kw('CALL'), 
      kw('CDCHKOFF'), kw('CDCHKON'), kw('CHDIR'), kw('CLS'), kw('CLREOL'), 
      kw('COLOR'), kw('CONFFLAG'), kw('CONFUNFLAG'), kw('COPY'), kw('CURSOR'), 
      kw('DBGLEVEL'), kw('DEC'), kw('DELAY'), kw('DELETE'), kw('DELUSER'),
      kw('DIR'), kw('DISPFILE'), kw('DISPSTR'), kw('DISPTEXT'), kw('DOWNLOAD'), 
      kw('DTROFF'), kw('DTRON'), kw('EVT'), kw('FAPPEND'), kw('FCLOSE'), 
      kw('FCLOSEALL'), kw('FCREATE'), kw('FDEFIN'), kw('FDEFOUT'), kw('FDGET'), 
      kw('FDPUT'), kw('FDPUTLN'), kw('FDPUTPAD'), kw('FDREAD'), kw('FDWRITE'), 
      kw('FGET'), kw('FLAG'), kw('FOPEN'), kw('FORWARD'), kw('FPUT'), 
      kw('FPUTLN'), kw('FPUTPAD'), kw('FREAD'), kw('FREALTUSER'), kw('FRESHLINE'), 
      kw('FSEEK'), kw('FWRITE'), kw('GETALTUSER'), kw('GETTOKEN'), kw('GETUSER'),
      kw('GOODBYE'), kw('HANGUP'), kw('INC'), kw('INPUT'), kw('INPUTCC'), 
      kw('INPUTDATE'), kw('INPUTINT'), kw('INPUTMONEY'), kw('INPUTSTR'), 
      kw('INPUTTEXT'), kw('INPUTTIME'), kw('INPUTYN'), kw('JOIN'), kw('KBDFILE'), 
      kw('KBDSTUFF'), kw('KBDSTRING'), kw('KBDFLUSH'), kw('KEYFLUSH'), kw('LANG'), 
      kw('LASTIN'), kw('LOG'), kw('MDMFLUSH'), kw('MKDIR'), kw('MOUSEREG'), 
      kw('MORE'), kw('MPRINT'), kw('MPRINTLN'), kw('NEWLINE'), kw('NEWLINES'), 
      kw('OPTEXT'), kw('PAGEOFF'), kw('PAGEON'), kw('POP'), kw('PRINT'),
      kw('PRINTLN'), kw('PRFOUND'), kw('PRFOUNDLN'), kw('PROMPTSTR'), kw('PUSH'), 
      kw('PUTUSER'), kw('PUTALTUSER'), kw('QUEST'), kw('RDUNET'), kw('RDUSYS'), 
      kw('REDIM'), kw('RENAME'), kw('RESETDISP'), kw('RESTSCRN'), kw('RMDIR'), 
      kw('SAVESCRN'), kw('SCRFILE'), kw('SEARCHINIT'), kw('SEARCHFIND'), 
      kw('SEARCHSTOP'), kw('SHELL'), kw('SORT'), kw('SOUND'), kw('SOUNDDELAY'), 
      kw('SPRINT'), kw('SPRINTLN'), kw('STACKABORT'), kw('STARTDISP'), kw('TOKENIZE'), 
      kw('TPAGET'), kw('TPAPUT'), kw('TPAREAD'), kw('TPAWRITE'), kw('TPACGET'), 
      kw('TPACPUT'), kw('TPACREAD'), kw('TPACWRITE'), kw('WAIT'), kw('WAITFOR'), 
      kw('WRUNET'), kw('WRUSYS'), kw('WRUSYSDOOR')
    ),

    // ---------- Builtin Functions ----------
    builtin_function: $ => choice(
      // Arithmetic / bitwise / numeric
      kw('ABS'), kw('BAND'), kw('BOR'), kw('BNOT'), kw('BXOR'), kw('RANDOM'), 
      kw('MAX'), kw('MIN'), kw('S2I'), kw('I2S'),
      // Conversion / formatting
      kw('FMTREAL'), kw('REPLACE'), kw('REPLACESTR'), kw('STRIP'), kw('STRIPATX'), 
      kw('STRIPSTR'), kw('LEFT'), kw('RIGHT'), kw('MID'), kw('LEN'), kw('LTRIM'), 
      kw('RTRIM'), kw('TRIM'), kw('LOWER'), kw('UPPER'), kw('MIXED'), kw('SPACE'),
      kw('TOBIGSTR'), kw('TOBOOLEAN'), kw('TOBYTE'), kw('TODATE'), kw('TODDATE'), 
      kw('TODREAL'), kw('TOEDATE'), kw('TOINTEGER'), kw('TOMONEY'), kw('TOREAL'), 
      kw('TOSBYTE'), kw('TOSWORD'), kw('TOTIME'), kw('TOUNSIGNED'), kw('TOWORD'), 
      kw('MKDATE'), kw('MKADDR'), kw('MEGANUM'),
      // Date/time functions
      kw('TIMEAP'), kw('DAY'), kw('MONTH'), kw('YEAR'), kw('HOUR'), kw('SEC'), kw('DOW'),
      // Input / keyboard / buffers
      kw('INKEY'), kw('TINKEY'), kw('KINKEY'), kw('NOCHAR'), kw('KBDBUFSIZE'), 
      kw('PPLBUFSIZE'), kw('KBDFILUSED'),
      // User / system / environment
      kw('CURCONF'), kw('CURSEC'), kw('CURCOLOR'), kw('CURUSER'), kw('U_LMR'), 
      kw('CONFREG'), kw('CONFEXP'), kw('CONFSEL'), kw('CONFSYS'), kw('CONFMW'), 
      kw('CONFALIAS'), kw('USERALIAS'), kw('CHATSTAT'), kw('ONLOCAL'),
      kw('PCBACCOUNT'), kw('PCBACCSTAT'), kw('PCBNODE'), kw('UN_NAME'), 
      kw('UN_CITY'), kw('UN_OPER'), kw('UN_STAT'),
      // Messaging / conference boundaries
      kw('LOMSGNUM'), kw('HIMSGNUM'),
      // Files / directories
      kw('EXIST'), kw('FILEINF'), kw('FERR'), kw('READLINE'),
      // Modem / carrier / call
      kw('CALLID'), kw('CALLNUM'), kw('CDON'), kw('CARRIER'),
      // Crypto / checksum
      kw('CRC32'),
      // Card / billing
      kw('CCTYPE'), kw('VALCC'), kw('FMTCC'),
      // Searching / pointer
      kw('INSTR'), kw('INSTRR'),
      // Flag related
      kw('FLAGCNT'),
      // Account/time event
      kw('EVTTIMEADJ'), kw('ADJTIME'),
      // Answer memory
      kw('DEFANS'), kw('LASTANS'),
      // Bit ops on values
      kw('ISBITSET'),
      // Math not listed earlier (NOTE: NOT is already handled in unary_expression)
      kw('AND'), kw('OR'), kw('XOR'),
      // Register (legacy placeholders)
      kw('REGAL'), kw('REGAH'), kw('REGBL'), kw('REGBH'), kw('REGCL'), kw('REGCH'), 
      kw('REGDL'), kw('REGDH'), kw('REGAX'), kw('REGBX'), kw('REGCX'), kw('REGDX'), 
      kw('REGSI'), kw('REGDI'), kw('REGF'), kw('REGCF'), kw('REGDS'), kw('REGES'),
      // Misc door / PPE
      kw('PPE_RNAME'), kw('PSA'), kw('GRAFMODE'), kw('GETX'), kw('GETY'), 
      kw('LANGEXT'), kw('PAGESTAT'),
      // Strip / mask validations
      kw('MASK_ALNUM'), kw('MASK_ALPHA'), kw('MASK_ASCII'), kw('MASK_FILE'), 
      kw('MASK_NUM'), kw('MASK_PATH'), kw('MASK_PWD'),
      // Validation
      kw('VALDATE'), kw('VALTIME'),
      // Misc
      kw('UNIXTIME')
    ),

    // ---------- Literals ----------
    string_literal: $ => seq(
      '"',
      repeat(choice(/[^"\\]/, /\\./)),
      '"'
    ),

    number_literal: $ => choice(
      $.hex_number,
      $.float_number,
      $.int_number
    ),

    int_number:   $ => /\d+/,
    float_number: $ => /\d+\.\d+/,
    hex_number:   $ => /0[xX][0-9A-Fa-f]+|[0-9A-Fa-f]+[hH]/,

    boolean_literal: $ => choice(kw('TRUE'), kw('FALSE')),

    at_color_code: $ => /@[xX][0-9A-Fa-f]{2}/,

    // ---------- Identifiers (now case-insensitive) ----------
    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/
  }
});