clap-errorkind-invalidvalue = Einer der Werte ist für ein Argument ungültig
clap-errorkind-unknown-arg = Unerwartetes Argument gefunden
clap-errorkind-invalid-subcmd = Unbekannter Unterbefehl
clap-errorkind-noeq = Gleichheitszeichen wird benötigt, wenn einem der Argumente Werte zugewiesen werden
clap-errorkind-value-validation = Ungültiger Wert für eines der Argumente
clap-errorkind-too-many-values = Unerwarteter Wert für ein Argument gefunden
clap-errorkind-too-few-values = Mehr Werte für ein Argument erforderlich
clap-errorkind-wrong-number-of-values = Zu viele oder zu wenige Werte für ein Argument
clap-errorkind-arg-conflict = Ein Argument kann nicht zusammen mit einem oder mehreren der anderen angegebenen Argumente verwendet werden
clap-errorkind-missing-required-arg = Ein oder mehrere erforderliche Argumente wurden nicht angegeben
clap-errorkind-missing-subcmd = Ein Unterbefehl ist erforderlich, aber es wurde keiner angegeben
clap-errorkind-invalid-utf8 = Ungültiges UTF-8 wurde in einem oder mehreren Argumenten erkannt
clap-dyn-errorkind-multipletimes = Das Argument '{ $arg }' kann nicht mehrmals verwendet werden
clap-dyn-errorkind-unrecognized-subcmd = Unbekannter Unterbefehl '{ $sub }'
clap-dyn-errorkind-argconflict = Das Argument '{ $arg }' kann nicht mit '{ $arg2 }' verwendet werden
clap-dyn-errorkind-subcmd-conflict = Der Unterbefehl '{ $arg }' kann nicht mit '{ $subcmd }' verwendet werden
clap-dyn-errorkind-conflict-other = {" eines oder mehrere der anderen angegebenen Argumente"}
clap-dyn-errorkind-no-eq = Gleichheitszeichen wird benötigt, wenn '{ $arg }' Werte zugewiesen werden
clap-dyn-errorkind-not-provided = Die folgenden erforderlichen Argumente wurden nicht angegeben:
clap-dyn-errorkind-subcmd-not-provided = '{ $sub }' erfordert einen Unterbefehl, aber es wurde keiner angegeben
clap-dyn-errorkind-required-arg-but-none = Ein Wert ist für '{ $arg }' erforderlich, aber es wurde keiner angegeben
clap-dyn-errorkind-too-many-values-no-more-expected = Unerwarteter Wert '{ $value }' für '{ $arg }' gefunden; es wurden keine weiteren erwartet
clap-help-tips = Für weitere Informationen versuchen Sie '{ $help }'.
clap-similar-exists-single = Ein ähnlicher { $context } existiert: '{ $possible }'
clap-similar-exists-multi = Einige ähnliche { $context } existieren:
clap-subcommand-context = { $multi -> 
    [true] Unterbefehle
    *[other] Unterbefehl
}
clap-argument-context = { $multi -> 
    [true] Argumente
    *[other] Argument
}
clap-possible-value-context = { $multi -> 
    [true] mögliche Werte
    *[other] möglicher Wert
}
clap-value-context = { $multi -> 
    [true] Werte
    *[other] Wert
}
clap-error-heading = Fehler
clap-tip-heading = Hinweis
clap-usage-heading = Aufruf:
clap-arguments-heading = Argumente
clap-options-heading = Optionen
clap-commands-heading = Befehle
clap-packages-value-name = PAKETE
clap-dyn-errorkind-unexpected-arg = Unerwartetes Argument '{ $arg }' gefunden
clap-dyn-errorkind-too-few-values = { $n ->
    [1] { $min_values } Werte werden von '{ $invalid_arg }' benötigt; es wurden nur '{ $actual_num_values }' angegeben
    *[other] { $min_values } Werte werden von '{ $invalid_arg }' benötigt; es wurde nur '{ $actual_num_values }' angegeben
}
clap-dyn-errorkind-wrong-number-of-values = { $n ->
    [1] { $num_values } Werte werden für '{ $invalid_arg }' benötigt, aber { $actual_num_values} wurden angegeben
    *[other] { $num_values } Werte werden von '{ $invalid_arg }' benötigt; aber '{ $actual_num_values }' wurde angegeben
}
clap-dyn-errorkind-value-validation = Ungültiger Wert '{ $invalid_value }' für '{ $invalid_arg }'
clap-help-help = Hilfe anzeigen (Zusammenfassung mit '-h')
clap-help-version = Version anzeigen