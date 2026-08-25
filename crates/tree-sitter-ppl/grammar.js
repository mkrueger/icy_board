/**
 * PPL (PCBoard Programming Language) grammar for tree-sitter.
 *
 * Covers PPL 1.00 - 4.01 as implemented by IcyBoard: the classic PCBoard
 * statements plus the 3.50 and 4.00 additions (REPEAT/LOOP, brackets, braces,
 * the dot operator, TYPE ... ENDTYPE, record literals and routine parameters).
 *
 * The language is case insensitive, so every keyword is a case insensitive
 * token. Built-in statements share a single token so that a statement head can
 * be told apart from a declaration; built-in functions stay plain identifiers
 * and are recognized in queries/highlights.scm instead.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/** Case insensitive pattern for a word. */
function ci(word) {
  return new RegExp(
    word
      .split('')
      .map(c => {
        if (/[a-zA-Z]/.test(c)) return `[${c.toLowerCase()}${c.toUpperCase()}]`;
        return /[.*+?^${}()|[\]\\]/.test(c) ? `\\${c}` : c;
      })
      .join(''),
  );
}

/** A keyword token, named after its upper case spelling. */
function kw(word) {
  return alias(token(ci(word)), word.toUpperCase());
}

/** `ENDIF` and `END IF` mean the same thing, and both lex as one token. */
function endKw(word) {
  return alias(token(seq(ci('END'), /[ \t]*/, ci(word))), 'END' + word.toUpperCase());
}

/** `ELSEIF` may also be written `ELSE IF`, `DEFAULT` also `CASE ELSE`. */
function pairKw(first, second, name) {
  return alias(token(seq(ci(first), /[ \t]*/, ci(second))), name);
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}

// Built-in statements, taken from STATEMENT_DEFINITIONS. Names the grammar
// handles as keywords (END, IF, LET, GOTO, GOSUB, RETURN, DECLARE, FUNCTION,
// PROCEDURE) and the internal opcodes (BEGIN, FEND, PCALL, PLACEHOLDER, STATIC)
// are left out.
const BUILTIN_STATEMENTS = [
  'ACCOUNT', 'ADDUSER', 'ADJBYTES', 'ADJDBYTES', 'ADJTBYTES', 'ADJTFILES', 'ADJTUBYTES', 'ADJTIME',
  'ALIAS', 'ANSIPOS', 'APPEND', 'BACKUP', 'BEEP', 'BITCLEAR', 'BITSET', 'BLT', 'BRAG', 'BROADCAST',
  'BYE', 'CALL', 'CDCHKOFF', 'CDCHKON', 'CHDIR', 'CHAT', 'CLOSECAP', 'CLREOL', 'CLS', 'COLOR',
  'COMMAND', 'CONFFLAG', 'CONFINFO', 'CONFUNFLAG', 'COPY',
  'DADD', 'DAPPEND', 'DBLANK', 'DBOTTOM', 'DCLOSE', 'DCLOSEALL', 'DCREATE', 'DDELETE', 'DFBLANK',
  'DFCOPY', 'DGET', 'DGO', 'DLOCK', 'DLOCKF', 'DLOCKG', 'DLOCKR', 'DNCLOSE', 'DNCLOSEALL',
  'DNCREATE', 'DNOPEN', 'DNEW', 'DOPEN', 'DPACK', 'DPUT', 'DRECALL', 'DSEEK', 'DSETALIAS', 'DSKIP',
  'DTAG', 'DTOP', 'DUNLOCK', 'DBGLEVEL', 'DEC', 'DEFCOLOR', 'DELUSER', 'DELAY', 'DELETE', 'DIR',
  'DISPFILE', 'DISPSTR', 'DISPTEXT', 'DOINTR', 'DOWNLOAD', 'DTROFF', 'DTRON', 'ERASE', 'EVAL',
  'FAPPEND', 'FCLOSE', 'FCLOSEALL', 'FCREATE', 'FDGET', 'FDOADDAKA', 'FDOADDORG', 'FDOQADD',
  'FDOQDEL', 'FDOQMOD', 'FDOWRAKA', 'FDOWRORG', 'FDPUT', 'FDPUTLN', 'FDPUTPAD', 'FDREAD',
  'FDWRITE', 'FDEFIN', 'FDEFOUT', 'FFLUSH', 'FGET', 'FOPEN', 'FPCLR', 'FPUT', 'FPUTLN', 'FPUTPAD',
  'FREAD', 'FREALTUSER', 'FREWIND', 'FSEEK', 'FWRITE', 'FLAG', 'FORWARD', 'FRESHLINE',
  'GETALTUSER', 'GETTOKEN', 'GETUSER', 'GOODBYE', 'GRAFMODE', 'HANGUP', 'INC', 'INPUT', 'INPUTCC',
  'INPUTDATE', 'INPUTINT', 'INPUTMONEY', 'INPUTSTR', 'INPUTTEXT', 'INPUTTIME', 'INPUTYN', 'JOIN',
  'KBDFILE', 'KBDFLUSH', 'KBDSTRING', 'KBDSTUFF', 'KBDCHKOFF', 'KBDCHKON', 'KEYFLUSH', 'KILLMSG',
  'LANG', 'LASTIN', 'LOG', 'MPRINT', 'MPRINTLN', 'MDMFLUSH', 'MESSAGE', 'MKDIR', 'MORE',
  'MOUSEREG', 'MOVEMSG', 'MSGTOFILE', 'NEWLINE', 'NEWLINES', 'NEWPWD', 'OPTEXT', 'OPENCAP',
  'PAGEOFF', 'PAGEON', 'POKE', 'POKEB', 'POKEDW', 'POKEW', 'POP', 'PRFOUND', 'PRFOUNDLN', 'PRINT',
  'PRINTLN', 'PROMPTSTR', 'PUSH', 'PUTALTUSER', 'PUTUSER', 'QUEST', 'QWKLIMITS', 'RDUSYS',
  'RDUNET', 'REDIM', 'REDIR', 'RECORDUSAGE', 'RENAME', 'RESETDISP', 'RESTSCRN', 'RMDIR', 'SPRINT',
  'SPRINTLN', 'STOP', 'SAVESCRN', 'SCRFILE', 'SEARCHFIND', 'SEARCHINIT', 'SEARCHSTOP', 'SENDMODEM',
  'SETBANKBAL', 'SETENV', 'SETLMR', 'SHELL', 'SHORTDESC', 'SHOWOFF', 'SHOWON', 'SORT', 'SOUND',
  'SOUNDDELAY', 'STACKABORT', 'STARTDISP', 'TPACGET', 'TPACPUT', 'TPACREAD', 'TPACWRITE', 'TPAGET',
  'TPAPUT', 'TPAREAD', 'TPAWRITE', 'TOKENIZE', 'USELMRS', 'VARADDR', 'VAROFF', 'VARSEG', 'WRUSYS',
  'WRUNET', 'WRUSYSDOOR', 'WAIT', 'WAITFOR', 'WEBREQUEST', 'ERRCLR',
];

// Types that may be written in a declaration, plus the read-only board objects.
const BUILTIN_TYPES = [
  'BIGSTR', 'BOOLEAN', 'BYTE', 'DATE', 'DDATE', 'DOUBLE', 'DREAL', 'DWORD', 'EDATE', 'FLOAT',
  'INTEGER', 'INT', 'LONG', 'MONEY', 'MSGAREAID', 'REAL', 'SBYTE', 'SDWORD', 'SHORT', 'STRING',
  'SWORD', 'TIME', 'UBYTE', 'UDWORD', 'UNSIGNED', 'UWORD', 'WORD',
  'AREA', 'BOARD', 'CONFERENCE', 'CONTACT', 'DIRECTORY', 'DOOR', 'ERROR', 'EVENT', 'FONT', 'GFX', 'MACROS', 'MARGINS', 'PALETTE', 'PASSWORD', 'SESSION', 'TERMINAL', 'TERMINFO', 'TERMINPUT',
  'ERRCODE', 'ERRKIND', 'EVENTKIND', 'GFXBACKEND', 'MOUSEACTION', 'MOUSEBUTTON', 'MOUSEMODE', 'MOUSETRACKING',
];

// Built-in constants, taken from BUILTIN_CONSTS. TRUE and FALSE are literals of
// their own, the names a statement or function already carries are left out.
const BUILTIN_CONSTANTS = [
  'ACC_CUR_BAL', 'ACC_MSGREAD', 'ACC_MSGWRITE', 'ACC_STAT', 'ACC_TIME', 'ATTACH_LIM_P',
  'ATTACH_LIM_U', 'AUTO', 'BELL', 'CHRG_CALL', 'CHRG_CHAT', 'CHRG_DOWNBYTES', 'CHRG_DOWNFILE',
  'CHRG_MSGCAP', 'CHRG_MSGECHOED', 'CHRG_MSGPRIVATE', 'CHRG_MSGREAD', 'CHRG_MSGWRITE',
  'CHRG_PEAKTIME', 'CHRG_TIME', 'CMAXMSGS', 'CRC_FILE', 'CRC_STR', 'CRED_SPECIAL', 'CRED_UPBYTES',
  'CRED_UPFILE', 'CUR_USER', 'DEB_CALL', 'DEB_CHAT', 'DEB_DOWNBYTES', 'DEB_DOWNFILE',
  'DEB_MSGCAP', 'DEB_MSGECHOED', 'DEB_MSGPRIVATE', 'DEB_MSGREAD', 'DEB_MSGWRITE', 'DEB_SPECIAL',
  'DEB_TIME', 'DEB_TPU', 'DEFS', 'ECHODOTS', 'ERASELINE', 'FCL', 'FIELDLEN', 'FNS', 'F_EXP',
  'F_MW', 'F_NET', 'F_REG', 'F_SEL', 'F_SYS',
  'GRAPH', 'GUIDE', 'HDR_ACTIVE', 'HDR_BLOCKS',
  'HDR_DATE', 'HDR_ECHO', 'HDR_FROM', 'HDR_MSGNUM', 'HDR_MSGREF', 'HDR_PWD', 'HDR_REPLY',
  'HDR_RPLYDATE', 'HDR_RPLYTIME', 'HDR_STATUS', 'HDR_SUBJ', 'HDR_TIME', 'HDR_TO', 'HIGHASCII',
  'LFAFTER', 'LFBEFORE', 'LOGIT', 'LOGITLEFT', 'MAXMSGS', 'NC', 'NEWBALANCE', 'NOCLEAR',
  'NO_USER', 'O_RD', 'O_RW', 'O_WR', 'PAY_UPBYTES', 'PAY_UPFILE', 'SEC_DROP', 'SEEK_CUR',
  'SEEK_END', 'SEEK_SET', 'STACKED', 'START_BAL', 'START_SESSION', 'STK_LIMIT', 'S_DB', 'S_DN',
  'S_DR', 'S_DW', 'UPCASE', 'WARNLEVEL', 'WORDWRAP', 'YESNO',
  'GFX_FLIP_NONE', 'GFX_FLIP_X', 'GFX_FLIP_Y',
  'KEY_ESCAPE', 'KEY_ENTER', 'KEY_TAB', 'KEY_BACKSPACE', 'KEY_DELETE',
  'KEY_UP', 'KEY_DOWN', 'KEY_RIGHT', 'KEY_LEFT', 'KEY_HOME', 'KEY_END',
  'KEY_PAGE_UP', 'KEY_PAGE_DOWN', 'KEY_INSERT',
];

const PREC = {
  OR: 1,
  COMPARE: 2,
  ADD: 3,
  MUL: 4,
  POW: 5,
  UNARY: 6,
  CALL: 7,
  MEMBER: 8,
};

module.exports = grammar({
  name: 'ppl',

  extras: $ => [/\s/, $.comment],

  word: $ => $.identifier,

  conflicts: $ => [
    [$._assignment_target, $._expression],
    [$._assignment_target, $._name_from_keyword],
    [$.predefined_call, $._name_from_keyword],
    [$.predefined_call],
    [$.return_statement],
    [$.procedure_call, $._expression],
    [$.member_call, $._expression],
  ],

  supertypes: $ => [$._statement, $._expression],

  rules: {
    source_file: $ => repeat($._top_level_item),

    _top_level_item: $ => choice(
      $.type_declaration,
      $.enum_declaration,
      $.function_declaration,
      $.procedure_declaration,
      $.function_definition,
      $.procedure_definition,
      $._statement,
    ),

    // ---------- Preprocessor ----------
    // Directives are written as comments so that a source using them still
    // reads as a comment to any older tool.
    _preprocessor_directive: $ => choice(
      $.define_directive,
      $.if_directive,
      $.elseif_directive,
      $.else_directive,
      $.endif_directive,
      $.usefuncs_directive,
      $.langversion_directive,
    ),

    // Says which language the file is written in, before anything else in it.
    langversion_directive: $ => seq(
      alias(token(seq(';', ci('$LANGVERSION'))), ';$LANGVERSION'),
      field('version', $.number_literal),
    ),

    define_directive: $ => prec.right(seq(
      alias(token(seq(';', ci('$DEFINE'))), ';$DEFINE'),
      field('name', $.identifier),
      optional(seq(optional('='), field('value', $._expression))),
    )),

    if_directive: $ => seq(
      alias(token(seq(';', ci('$IF'))), ';$IF'),
      field('condition', $._expression),
    ),

    elseif_directive: $ => seq(
      alias(token(seq(';', choice(ci('$ELSEIF'), ci('$ELIF')))), ';$ELSEIF'),
      field('condition', $._expression),
    ),

    else_directive: $ => alias(token(seq(';', ci('$ELSE'))), ';$ELSE'),
    endif_directive: $ => alias(token(seq(';', ci('$ENDIF'))), ';$ENDIF'),
    usefuncs_directive: $ => alias(token(seq(';', ci('$USEFUNCS'), /[^\n]*/)), ';$USEFUNCS'),

    // `;#NAME` is replaced by the value of a preprocessor variable.
    substitution: $ => token(seq(';#', /[A-Za-z_][A-Za-z0-9_]*/)),

    // ---------- Declarations ----------
    type_declaration: $ => seq(
      kw('TYPE'),
      field('name', $.identifier),
      repeat($.field_declaration),
      endKw('TYPE'),
    ),

    enum_declaration: $ => seq(
      kw('ENUM'),
      field('name', $.identifier),
      repeat($.enum_variant),
      endKw('ENUM'),
    ),

    enum_variant: $ => seq(
      field('name', $.identifier),
      optional(seq('=', field('value', $._expression))),
      optional(','),
    ),

    field_declaration: $ => seq(
      field('type', $._type),
      commaSep1(field('name', $.identifier)),
    ),

    function_declaration: $ => seq(
      kw('DECLARE'),
      kw('FUNCTION'),
      field('name', $.identifier),
      $.parameter_list,
      field('return_type', $._type),
    ),

    procedure_declaration: $ => seq(
      kw('DECLARE'),
      kw('PROCEDURE'),
      field('name', $.identifier),
      $.parameter_list,
    ),

    function_definition: $ => seq(
      kw('FUNCTION'),
      field('name', $.identifier),
      $.parameter_list,
      field('return_type', $._type),
      field('body', repeat($._statement)),
      choice(endKw('FUNC'), endKw('FUNCTION')),
    ),

    procedure_definition: $ => seq(
      kw('PROCEDURE'),
      field('name', $.identifier),
      $.parameter_list,
      field('body', repeat($._statement)),
      choice(endKw('PROC'), endKw('PROCEDURE')),
    ),

    parameter_list: $ => seq('(', commaSep($._parameter), ')'),

    _parameter: $ => choice($.parameter, $.function_parameter, $.procedure_parameter),

    parameter: $ => seq(
      optional(kw('VAR')),
      field('type', $._type),
      field('name', $.identifier),
      optional($.dimensions),
    ),

    // A routine may be passed to a routine since 3.50.
    function_parameter: $ => seq(
      kw('FUNCTION'),
      field('name', $.identifier),
      $.parameter_list,
      field('return_type', $._type),
    ),

    procedure_parameter: $ => seq(
      kw('PROCEDURE'),
      field('name', $.identifier),
      $.parameter_list,
    ),

    _type: $ => choice($.builtin_type, alias($.identifier, $.type_identifier)),

    builtin_type: $ => choice(...BUILTIN_TYPES.map(t => kw(t))),

    // ---------- Statements ----------
    _statement: $ => choice(
      $.variable_declaration,
      $.const_declaration,
      $.assignment_statement,
      $.if_statement,
      $.if_block,
      $.while_statement,
      $.while_block,
      $.repeat_statement,
      $.loop_statement,
      $.for_statement,
      $.select_statement,
      $.goto_statement,
      $.gosub_statement,
      $.on_error_statement,
      $.return_statement,
      $.break_statement,
      $.continue_statement,
      $.end_statement,
      $.exit_statement,
      $.block,
      $.label,
      $.predefined_call,
      $.procedure_call,
      $.member_call,
      $._preprocessor_directive,
    ),

    variable_declaration: $ => seq(
      field('type', $._type),
      commaSep1($.variable_declarator),
    ),

    // The value has to be one the compiler can work out.
    const_declaration: $ => seq(
      kw('CONST'),
      field('type', $._type),
      field('name', $.identifier),
      '=',
      field('value', $._expression),
    ),

    variable_declarator: $ => prec.right(seq(
      field('name', $.identifier),
      optional($.dimensions),
      optional(seq('=', field('value', choice($._expression, $.array_initializer)))),
    )),

    dimensions: $ => seq('(', commaSep1($._expression), ')'),

    array_initializer: $ => seq('{', commaSep($._expression), '}'),

    assignment_statement: $ => seq(
      optional(kw('LET')),
      field('left', $._assignment_target),
      field('operator', choice('=', '+=', '-=', '*=', '/=', '%=', '&=', '|=')),
      field('right', $._expression),
    ),

    // A variable may carry the name of a built-in statement, constant or type, so those
    // spellings have to be accepted on the left of an assignment as well.
    _assignment_target: $ => choice(
      $.identifier,
      alias($.builtin_statement, $.identifier),
      alias($.builtin_constant, $.identifier),
      alias($.builtin_type, $.identifier),
      $.member_access,
      $.index_expression,
      $.call_expression,
    ),
    if_statement: $ => seq(
      kw('IF'),
      field('condition', $._expression),
      field('consequence', $._statement),
    ),

    if_block: $ => seq(
      kw('IF'),
      field('condition', $._expression),
      kw('THEN'),
      field('consequence', repeat($._statement)),
      repeat($.elseif_clause),
      optional($.else_clause),
      endKw('IF'),
    ),

    elseif_clause: $ => seq(
      choice(kw('ELSEIF'), pairKw('ELSE', 'IF', 'ELSEIF')),
      field('condition', $._expression),
      optional(kw('THEN')),
      field('consequence', repeat($._statement)),
    ),

    else_clause: $ => seq(kw('ELSE'), field('body', repeat($._statement))),

    while_statement: $ => seq(
      kw('WHILE'),
      field('condition', $._expression),
      field('body', $._statement),
    ),

    while_block: $ => seq(
      kw('WHILE'),
      field('condition', $._expression),
      kw('DO'),
      field('body', repeat($._statement)),
      endKw('WHILE'),
    ),

    repeat_statement: $ => seq(
      kw('REPEAT'),
      field('body', repeat($._statement)),
      kw('UNTIL'),
      field('condition', $._expression),
    ),

    loop_statement: $ => seq(
      kw('LOOP'),
      field('body', repeat($._statement)),
      endKw('LOOP'),
    ),

    for_statement: $ => prec.right(seq(
      kw('FOR'),
      field('variable', $.identifier),
      '=',
      field('start', $._expression),
      kw('TO'),
      field('end', $._expression),
      optional(seq(kw('STEP'), field('step', $._expression))),
      field('body', repeat($._statement)),
      choice(kw('NEXT'), endKw('FOR')),
      optional(field('variable_end', $.identifier)),
    )),

    select_statement: $ => seq(
      kw('SELECT'),
      optional(kw('CASE')),
      field('value', $._expression),
      repeat($.case_clause),
      optional($.default_clause),
      endKw('SELECT'),
    ),

    case_clause: $ => seq(
      kw('CASE'),
      commaSep1($.case_label),
      optional(':'),
      field('body', repeat($._statement)),
    ),

    case_label: $ => prec.right(choice(
      seq($._expression, '..', $._expression),
      $._expression,
    )),

    default_clause: $ => seq(
      choice(kw('DEFAULT'), pairKw('CASE', 'ELSE', 'DEFAULT')),
      optional(':'),
      field('body', repeat($._statement)),
    ),

    goto_statement: $ => seq(kw('GOTO'), field('label', $.identifier)),
    gosub_statement: $ => seq(kw('GOSUB'), field('label', $.identifier)),

    // ON ERROR is two words, ONERROR one; both name the same statement.
    on_error_statement: $ => seq(
      choice(kw('ONERROR'), seq(kw('ON'), kw('ERROR'))),
      choice(
        kw('OFF'),
        seq(kw('GOTO'), field('label', $.identifier)),
        seq(kw('GOSUB'), field('label', $.identifier)),
        field('handler', $.identifier),
      ),
    ),

    // A value after RETURN needs 3.50. Without one the next line stands on its
    // own, which is what old sources mean.
    return_statement: $ => choice(
      kw('RETURN'),
      prec.dynamic(-1, seq(kw('RETURN'), field('value', $._expression))),
    ),

    break_statement: $ => kw('BREAK'),
    continue_statement: $ => kw('CONTINUE'),
    end_statement: $ => prec(-1, kw('END')),

    // What END was before 400 took that word for the block terminator.
    exit_statement: $ => kw('EXIT'),

    // Language version 400 made BEGIN ... END a block; before that BEGIN was a
    // pseudo label and the END below it the statement that stops a program.
    block: $ => seq(
      kw('BEGIN'),
      field('body', repeat($._statement)),
      kw('END'),
    ),

    // A label is one token, the way the compiler lexes it: no space after ':'.
    label: $ => token(seq(':', /[A-Za-z_][A-Za-z0-9_]*/)),

    predefined_call: $ => seq(
      field('name', $.builtin_statement),
      optional(commaSep1(field('argument', $._expression))),
    ),

    procedure_call: $ => seq(
      field('name', $.identifier),
      $.argument_list,
    ),

    member_call: $ => seq(
      field('member', $.member_access),
      $.argument_list,
    ),

    argument_list: $ => choice(
      seq('(', commaSep(field('argument', $._expression)), ')'),
      seq('[', commaSep(field('argument', $._expression)), ']'),
    ),

    // ---------- Expressions ----------
    _expression: $ => choice(
      $.identifier,
      $.builtin_constant,
      $._name_from_keyword,
      $.constant,
      $.substitution,
      $.parenthesized_expression,
      $.unary_expression,
      $.binary_expression,
      $.call_expression,
      $.index_expression,
      $.member_access,
      $.record_literal,
    ),

    // `LANG`, `STRING` and friends name a statement or a type and a function or
    // constant at the same time. Reading one as a value is the last resort, so
    // that a statement of its own is preferred where both would parse.
    _name_from_keyword: $ => prec.dynamic(-1, choice(
      alias($.builtin_statement, $.identifier),
      alias($.builtin_type, $.identifier),
    )),

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    unary_expression: $ => prec.right(PREC.UNARY, seq(
      field('operator', choice('-', '+', '!')),
      field('operand', $._expression),
    )),

    binary_expression: $ => {
      const table = [
        [PREC.OR, choice('&&', '&', '||', '|')],
        [PREC.COMPARE, choice('==', '=', '!=', '<>', '><', '<=', '=<', '>=', '=>', '<', '>')],
        [PREC.ADD, choice('+', '-')],
        [PREC.MUL, choice('*', '/', '%')],
      ];
      return choice(
        ...table.map(([precedence, operator]) => prec.left(precedence, seq(
          field('left', $._expression),
          field('operator', operator),
          field('right', $._expression),
        ))),
        prec.right(PREC.POW, seq(
          field('left', $._expression),
          field('operator', '^'),
          field('right', $._expression),
        )),
      );
    },

    call_expression: $ => prec(PREC.CALL, seq(
      field('function', $._expression),
      '(',
      commaSep(field('argument', $._expression)),
      ')',
    )),

    index_expression: $ => prec(PREC.CALL, seq(
      field('array', $._expression),
      '[',
      commaSep1(field('index', $._expression)),
      ']',
    )),

    // A type name may stand in expression position, naming an enum member or the one
    // value a board object has, so it is an object here as well as a declaration type.
    member_access: $ => prec(PREC.MEMBER, seq(
      field('object', choice($._expression, $.builtin_type)),
      '.',
      field('member', choice($.identifier, $.builtin_constant)),
    )),

    // `Point { X = 1, Y = 2 }` builds a record without temporary assignments.
    record_literal: $ => prec(PREC.MEMBER, seq(
      field('type', alias($.identifier, $.type_identifier)),
      '{',
      commaSep($.record_literal_field),
      '}',
    )),

    record_literal_field: $ => seq(
      field('name', $.identifier),
      '=',
      field('value', $._expression),
    ),

    // ---------- Terminals ----------
    // One token for every built-in statement name. A lexical precedence would
    // beat a longer keyword such as DECLARE, so the names carry none.
    builtin_statement: $ => token(choice(...BUILTIN_STATEMENTS.map(s => ci(s)))),

    // The built-in constants are their own token so that highlighting does not
    // need a case insensitive query predicate. Built-in functions do not need
    // one: a call is recognized by its parentheses.
    builtin_constant: $ => token(choice(...BUILTIN_CONSTANTS.map(s => ci(s)))),

    constant: $ => choice(
      $.string_literal,
      $.number_literal,
      $.money_literal,
      $.color_code,
      $.boolean_literal,
    ),

    // A doubled quote is a quote, there is no backslash escape.
    string_literal: $ => token(seq('"', repeat(choice(/[^"]/, '""')), '"')),

    number_literal: $ => token(choice(
      /\d+\.\d+/,
      /\d[0-9A-Fa-f]*[hH]/,
      /[01]+[bB]/,
      /[0-7]+[oO]/,
      /\d+[dD]/,
      /\d+/,
    )),

    money_literal: $ => token(seq('$', /\d+(\.\d+)?/)),

    color_code: $ => token(seq('@', /[xX]/, /[0-9A-Fa-f]{2}/)),

    boolean_literal: $ => choice(kw('TRUE'), kw('FALSE')),

    comment: $ => token(choice(
      seq(';', /[^$#\n][^\n]*/),
      seq(';', /\r?\n/),
      seq("'", /[^\n]*/),
    )),

    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/,
  },
});
