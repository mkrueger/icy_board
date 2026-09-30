clap-errorkind-invalidvalue = one of the values isn't valid for an argument
clap-errorkind-unknown-arg = unexpected argument found
clap-errorkind-invalid-subcmd = unrecognized subcommand
clap-errorkind-noeq = equal is needed when assigning values to one of the arguments
clap-errorkind-value-validation = invalid value for one of the arguments
clap-errorkind-too-many-values = unexpected value for an argument found
clap-errorkind-too-few-values = more values required for an argument
clap-errorkind-wrong-number-of-values = too many or too few values for an argument
clap-errorkind-arg-conflict = an argument cannot be used with one or more of the other specified arguments
clap-errorkind-missing-required-arg = one or more required arguments were not provided
clap-errorkind-missing-subcmd = a subcommand is required but one was not provided
clap-errorkind-invalid-utf8 = invalid UTF-8 was detected in one or more arguments
clap-dyn-errorkind-multipletimes = the argument '{ $arg }' cannot be used multiple times
clap-dyn-errorkind-unrecognized-subcmd = unrecognized subcommand '{ $sub }'
clap-dyn-errorkind-argconflict = the argument '{ $arg }' cannot be used with '{ $arg2 }'
clap-dyn-errorkind-subcmd-conflict = the subcommand '{ $arg }' cannot be used with '{ $subcmd }'
clap-dyn-errorkind-conflict-other = {" one or more of the other specified arguments"}
clap-dyn-errorkind-no-eq = equal sign is needed when assigning values to '{ $arg }'
clap-dyn-errorkind-not-provided = the following required arguments were not provided:
clap-dyn-errorkind-subcmd-not-provided = '{ $sub }' requires a subcommand but one was not provided
clap-dyn-errorkind-required-arg-but-none = a value is required for '{ $arg }' but none was supplied
clap-dyn-errorkind-too-many-values-no-more-expected = unexpected value '{ $value }' for '{ $arg }' found; no more were expected
clap-help-tips = For more information, try '{ $help }'.
clap-similar-exists-single = a similar { $context } exists: '{ $possible }'
clap-similar-exists-multi = some similar { $context }s exist:
clap-subcommand-context = { $multi -> 
    [true] subcommands
    *[other] subcommand
}
clap-argument-context = { $multi -> 
    [true] arguments
    *[other] argument
}
clap-possible-value-context = { $multi -> 
    [true] possible values
    *[other] possible value
}
clap-value-context = { $multi -> 
    [true] values
    *[other] value
}
clap-error-heading = error
clap-tip-heading = tip
clap-usage-heading = Usage:
clap-arguments-heading = Arguments
clap-options-heading = Options
clap-commands-heading = Commands
clap-packages-value-name = PACKAGES
clap-dyn-errorkind-unexpected-arg = unexpected argument '{ $arg }' found
clap-dyn-errorkind-too-few-values = { $n ->
    [1] { $min_values } values required by '{ $invalid_arg }'; only '{ $actual_num_values }' were provided
    *[other] { $min_values } values required by '{ $invalid_arg }'; only '{ $actual_num_values }' was provided
}
clap-dyn-errorkind-wrong-number-of-values = { $n ->
    [1] { $num_values } values required for '{ $invalid_arg }' but { $actual_num_values} were provided
    *[other] { $num_values } values required by '{ $invalid_arg }'; but '{ $actual_num_values }' was provided
}
clap-dyn-errorkind-value-validation = invalid value '{ $invalid_value }' for '{ $invalid_arg }'
clap-help-help = Print help (see a summary with '-h')
clap-help-version = Print version
