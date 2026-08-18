error_cmd_line_label = Fehler:
error_board_config_not_found = IcyBoard-Konfiguration nicht gefunden: { $path }
error_board_config_help =
    Ein angegebener Pfad wird genau so verwendet. Ohne Pfad sucht IcyBoard
    icboard.toml im aktuellen Verzeichnis und danach unter ICB_PATH.

    Verwendung: { $program } [Optionen] [DATEI|VERZEICHNIS]
    Board erstellen: icbsetup create mybbs
    Danach starten: icboard mybbs
    Kommandohilfe: { $program } --help
    Anleitung: https://github.com/mkrueger/icy_board/blob/main/docs/gettingstarted.md
error_input_file_not_found = Eingabedatei nicht gefunden: { $path }
error_input_file_help =
    Verwendung: { $program } [Optionen] DATEI
    Datei erstellen: { $program } --create { $path }
    Kommandohilfe: { $program } --help
error_parent_board_config_not_found = Keine icboard.toml gefunden für: { $path }
error_parent_board_config_help =
    { $program } sucht icboard.toml im Verzeichnis der Datei und darüber.

    Board erstellen: icbsetup create mybbs
    Kommandohilfe: { $program } --help
    Anleitung: https://github.com/mkrueger/icy_board/blob/main/docs/gettingstarted.md
run_ppe_completed = Ausführung abgeschlossen - beliebige Taste zum Beenden

option_not_read_yet = wird vom Board noch nicht ausgewertet
option_imported_only = stammt aus dem PCBoard-Import und wird nicht ausgewertet


yes=Ja
no=Nein

icbtext_save_changes=Änderungen speichern?
icbtext_edit_title=Eintrag #{ $number } Bearbeiten
icbtext_edit_original_text_title=Originaltext:
icbtext_edit_preview_text_title=Vorschau:
icbtext_edit_edit_text_title=Bearbeiten:
icbtext_edit_hard_space_info=Tilde (~) für abschließende Leerzeichen am Stringende verwenden.
icbtext_edit_justify_left=Links
icbtext_edit_justify_right=Rechts
icbtext_edit_justify_center=Zentriert
icbtext_edit_justify_title=Ausrichtung: { $justify }
icbtext_edit_record_length_title=Eintragslänge: { $number } Zeichen
icbtext_edit_style=Stil:

icbtext_filter_title=Filter
icbtext_filter_text=Zeige mit '{ $filter }' gefilterte Einträge
icbtext_no_entries=Keine Einträge gefunden

icbtext_jump_to_title=Zu Eintrag # springen

icbtext_style_plain = Kein
icbtext_style_red = Rot
icbtext_style_green = Grün
icbtext_style_yellow = Gelb
icbtext_style_blue = Blau
icbtext_style_purple = Lila
icbtext_style_cyan = Cyan
icbtext_style_white = Weiß

icbtext_tab_record=Einträge
icbtext_tab_about=Über
icb_setup_save_failed=Speichern fehlgeschlagen: { $error }

key_desc_quit=Beenden
key_desc_back=Zurück
key_desc_next_prev_style=Stil vor/zurück
key_desc_restore=Reset
key_desc_accept=Übernehmen
key_desc_cancel=Abbrechen
key_desc_filter=Filter
key_desc_jump=Springe
key_desc_edit=Bearbeiten