# Programmrahmen und gemeinsame Info-Seite
app_icbsetup = IcyBoard-Einrichtung
app_icbsm = IcyBoard-Systemverwaltung
app_mkicbtxt = ICBTEXT-Dateigenerator/-Editor
app_mkicbmnu = MNU-Dateieditor
app_file_title = { $application } ({ $path })
tui_tab_main = Hauptmenü
tui_tab_about = Über
tui_tab_general = Allgemein
tui_tab_commands = Befehle
about_version = { $application } v{ $version }
about_author = entwickelt { $year } von { $author } als Teil des icy_board-Projekts
about_website = Besuche { $url }
about_updates = für die aktuelle Version und Diskussionen

# Menüeditor
mnu_editor_title = Titel
mnu_editor_title_status = Titel des Menüs eingeben.
mnu_editor_display_file = Anzeigedatei
mnu_editor_display_file_status = Datei für den Menühintergrund.
mnu_editor_help_file = Hilfedatei
mnu_editor_help_file_status = Hilfedatei, die angezeigt wird.
mnu_editor_menu_type = Menütyp
mnu_editor_menu_type_status = Art des Menüs.
mnu_editor_prompt = Eingabezeile
mnu_editor_prompt_status = Eingabeaufforderung für das Menü.
mnu_editor_type_hotkey = Direktwahl
mnu_editor_type_lightbar = Auswahlbalken
mnu_editor_type_command = Befehlseingabe
mnu_editor_display = Anzeige
mnu_editor_display_text = Anzeigetext
mnu_editor_display_text_status = Angezeigter Text.
mnu_editor_highlighted_text = Markierter Text
mnu_editor_highlighted_text_status = Text bei hervorgehobenem Eintrag.
mnu_editor_position = Position
mnu_editor_position_status = Position des Eintrags auf dem Menübildschirm.
mnu_editor_keyword_status = Schlüsselwort zum Auswählen dieses Eintrags.
mnu_editor_autorun = Automatik
mnu_editor_autorun_status = Zeitpunkt der automatischen Ausführung des Eintrags.
mnu_editor_autorun_disabled = Deaktiviert
mnu_editor_autorun_first = Beim ersten Laden
mnu_editor_autorun_every = Vor der Anzeige
mnu_editor_autorun_after = Nach der Anzeige
mnu_editor_autorun_loop = Wiederholt
mnu_editor_time = Zeit
mnu_editor_time_status = Automatische Ausführung nach einer bestimmten Zeit.
mnu_editor_security_status = Erforderliche Sicherheitsstufe für diesen Eintrag.
mnu_editor_parameter_status = Argument, das dem Befehl übergeben wird.
mnu_editor_run_on_selection = Bei Markierung
mnu_editor_run_on_selection_status = Beim Hervorheben des Eintrags ausführen statt beim Bestätigen.
mnu_editor_command_title = Befehls-ID { $id }
mnu_editor_edit_action = Aktion bearbeiten

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

# Anruf-Warteschirm
call_wait_screen_sysop_page = SYSOP-RUF: Node { $node } - { $user } ({ $count } aktiv)
call_wait_screen_unknown_caller = Unbekannter Anrufer


yes=Ja
no=Nein

icbtext_save_changes=Änderungen speichern?
icbtext_edit_title=Eintrag #{ $number } bearbeiten
icbtext_edit_original_text_title=Originaltext:
icbtext_edit_preview_text_title=Vorschau:
icbtext_edit_edit_text_title=Bearbeiten:
icbtext_edit_hard_space_info=Tilde (~) für feste Leerzeichen am Textende verwenden.
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
key_desc_restore=Wiederherstellen
key_desc_accept=Übernehmen
key_desc_cancel=Abbrechen
key_desc_filter=Filter
key_desc_jump=Springen
key_desc_edit=Bearbeiten
configuration_options_upload_processing=Upload-Verarbeitung

upload_processing_title=Upload-Verarbeitung
upload_processing_group_publication=Veröffentlichung und Quarantäne
upload_processing_group_advertising=Werbung entfernen und eigene Datei
upload_processing_group_zip=ZIP-Ausgabe
upload_processing_group_scanner=Virenscanner
upload_processing_group_limits=Ressourcen- und Sicherheitsgrenzen
upload_processing_publish_policy=Veröffentlichung
upload_processing_publish_policy-status=Legt fest, wann verarbeitete Uploads sichtbar werden.
upload_processing_publish_policy-help=Sofort veröffentlicht Uploads ohne diese Verarbeitung. Nach erfolgreicher Verarbeitung werden Dateien automatisch veröffentlicht. Manuelle Sysop-Freigabe hält verarbeitete Dateien bis zur Freigabe in Quarantäne.
upload_processing_policy_immediate=Sofort
upload_processing_policy_after_processing=Nach erfolgreicher Verarbeitung
upload_processing_policy_manual_approval=Manuelle Sysop-Freigabe
upload_processing_notify_sysop=Sysop per Mail benachrichtigen
upload_processing_notify_sysop-status=Sendet für jeden angenommenen Upload eine private lokale Nachricht.
upload_processing_notify_sysop-help=Die Nachricht enthält Datei, Uploader, Verarbeitungsstatus und Beschreibung. Ein Mailfehler wird protokolliert, lehnt den Upload aber nicht ab.
upload_processing_remove_advertisements=Werbung entfernen
upload_processing_remove_advertisements-status=Entfernt erkannte Werbedateien, Werbenachspann in Beschreibungen und bekannte ZIP-Werbekommentare.
upload_processing_remove_advertisements-help=
    Ein Schalter entfernt erkannte Werbedateien anhand von Prüfsummen, Dateinamen
    und Textmustern, begrenzte Werbeblöcke (einschließlich BBS- und Courier-Nachspann)
    aus kanonischen Beschreibungsdateien sowie bekannte ZIP-Werbekommentare anhand von Regeln.
    Die Beschreibungsbereinigung ist intern auf 8 Durchläufe begrenzt; verbleibende Treffer erfordern danach eine Prüfung.
    Unbekannte ZIP-Kommentare bleiben erhalten, sofern kein eigener ZIP-Kommentar sie ausdrücklich ersetzt.
    Kommentare anderer Archivformate werden nicht ausgelesen und bei der ZIP-Konvertierung nicht übernommen.
upload_processing_repack_zip=Als ZIP neu packen
upload_processing_repack_zip-status=Schreibt akzeptierte Archive als ZIP-Dateien neu.
upload_processing_repack_zip-help=Lesbare Archivformate werden mit dem direkt folgenden Kompressionsgrad nach ZIP konvertiert. Kommentare anderer Formate werden nicht ausgelesen und bei der Konvertierung nicht übernommen.
upload_processing_rules=Werberegeln
upload_processing_rules-status=TOML-Datei mit Regeln für Werbedateien, Beschreibungsblöcke und ZIP-Kommentare.
upload_processing_rules-help=Neue Muster zunächst nur protokollieren und Treffer prüfen, bevor automatisch bereinigt wird.
upload_processing_quarantine=Quarantäneverzeichnis
upload_processing_quarantine-status=Privates Verzeichnis für Uploads in Verarbeitung oder Freigabe.
upload_processing_quarantine-help=Dieses Verzeichnis muss außerhalb aller öffentlichen Dateibereiche liegen.
upload_processing_advertisement_file=Eigene Werbedatei
upload_processing_advertisement_file-status=Eine statische Datei oder ein vertrauenswürdiger PPE-Generator; leer deaktiviert das Einfügen.
upload_processing_advertisement_file-help=
    Leer lassen, um das Einfügen zu deaktivieren. Ein normaler Pfad fügt eine statische Datei unter ihrem Basisnamen ein.
    Leerzeichen und Semikolons sind wörtliche Bestandteile dieses einzelnen Pfads, keine Listentrenner.
    Die Erweiterung .ppe (Groß-/Kleinschreibung beliebig) startet einen Generator, der null oder eine Datei
    im übergebenen temporären Ausgabeverzeichnis (Parameter 1) erzeugen darf. Parameter 2 ist der ursprüngliche
    Archivbasisname. Die Argumente werden wörtlich übergeben und mit GETTOKEN gelesen.
    Es steht kein Entpackverzeichnis zur Verfügung, und es gibt keinen Kontext eines angemeldeten Anrufers.
    Nur vertrauenswürdige SysOp-PPE-Programme verwenden: Die Ausführung erfolgt ohne Sandbox.
    Die Generatorlaufzeit ist auf 30 Sekunden begrenzt; die Ausgabegröße auf den kleineren Wert
    von max_member_size und 16 MiB. Bereits vorhandene Einträge mit dem Ausgabebasisnamen erfordern eine Prüfung.
    Beschreibungsdateien (FILE_ID.DIZ, FILE_ID.ANS, FILE_ID.PCB, DESC.SDI) dürfen nicht hinzugefügt oder ersetzt werden.
    Der konfigurierte Virenscanner läuft nach dem Einfügen und der Archivverarbeitung.
upload_processing_replacement_comment=Eigener ZIP-Kommentar
upload_processing_replacement_comment-status=Optionaler Boardtext für den ZIP-Archivkommentar.
upload_processing_replacement_comment-help=Ein nichtleerer Text ersetzt den ZIP-Ausgabekommentar ausdrücklich, auch unbekannte Kommentare. Leer lassen, um unbekannte ZIP-Kommentare zu erhalten; bekannte Werbekommentare werden nur bei aktiviertem „Werbung entfernen“ gelöscht. Diese Einstellung schreibt nur ZIP-Kommentare, keine Kommentare anderer Formate.
upload_processing_compression=ZIP-Kompressionsgrad
upload_processing_compression-status=Deflate-Kompressionsgrad von 0 bis 9.
upload_processing_compression-help=Höhere Werte können Platz sparen, benötigen aber mehr Rechenzeit.
upload_processing_max_members=Maximale Archivdateien
upload_processing_max_members-status=Archive mit mehr Einträgen werden zur Prüfung vorgemerkt.
upload_processing_max_members-help=Begrenzt CPU- und Metadatenarbeit durch Archive mit sehr vielen Einträgen.
upload_processing_max_member_size=Maximale Eintragsgröße
upload_processing_max_member_size-status=Größte erlaubte entpackte Größe eines Archiveintrags.
upload_processing_max_member_size-help=Der Wert wird in Bytes angegeben.
upload_processing_max_expanded_size=Maximale entpackte Größe
upload_processing_max_expanded_size-status=Größte erlaubte entpackte Gesamtgröße eines Archivs.
upload_processing_max_expanded_size-help=Der Wert wird in Bytes angegeben und schützt vor Archivbomben.
upload_processing_max_ratio=Maximales Kompressionsverhältnis
upload_processing_max_ratio-status=Größtes erlaubtes Verhältnis von entpackter zu gepackter Größe.
upload_processing_max_ratio-help=Einträge über diesem Verhältnis werden vor dem Entpacken zur Prüfung vorgemerkt.
upload_processing_scanner_enabled=Virenscanner aktivieren
upload_processing_scanner_enabled-status=Startet nach der Archivverarbeitung einen externen Scanner.
upload_processing_scanner_enabled-help=Das Programm wird direkt und ohne Kommando-Shell gestartet.
upload_processing_scanner_executable=Scanner-Programm
upload_processing_scanner_executable-status=Programmname oder absoluter Pfad, zum Beispiel clamscan.
upload_processing_scanner_executable-help=Bei einem reinen Programmnamen wird PATH durchsucht.
upload_processing_scanner_arguments=Scanner-Argumente
upload_processing_scanner_arguments-status=Durch Semikolon getrennte Argumente mit genau einem eigenständigen { "{file}" }.
upload_processing_scanner_arguments-help=Es gibt keine Shell-Ersetzung. Das exakte Argument { "{file}" } wird durch den Quarantänepfad ersetzt.
upload_processing_scanner_arguments-invalid=Scanner-Argumente müssen genau ein eigenständiges { "{file}" }-Argument enthalten.
upload_processing_scanner_timeout=Scanner-Timeout in Sekunden
upload_processing_scanner_timeout-status=Maximale Laufzeit eines Scanner-Prozesses.
upload_processing_scanner_timeout-help=Ein Timeout gilt als technischer Fehler und erfordert eine Prüfung.
upload_processing_scanner_clean_code=Exitcode für sauber
upload_processing_scanner_clean_code-status=Exitcode, wenn keine Schadsoftware gefunden wurde.
upload_processing_scanner_clean_code-help=ClamAV verwendet Exitcode 0 für eine saubere Datei.
upload_processing_scanner_infected_code=Exitcode für infiziert
upload_processing_scanner_infected_code-status=Exitcode, wenn Schadsoftware gefunden wurde.
upload_processing_scanner_infected_code-help=ClamAV verwendet Exitcode 1 für eine infizierte Datei. Andere Codes gelten als technische Fehler.

exit_icy_board_msg = Vielen Dank, dass Sie die professionelle BBS-Software { $name } verwenden!

# Anruf-Warteschirm
call_wait_screen_user_button_busy=Benutzer – Besetzt
call_wait_screen_user_button_busy_descr=Als normaler Benutzer anmelden. Anrufer erhalten ein Besetztzeichen.
call_wait_screen_sysop_button_busy=Sysop – Besetzt
call_wait_screen_sysop_button_busy_descr=Als Sysop anmelden. Anrufer erhalten ein Besetztzeichen.
call_wait_screen_dos_button_busy=Shell – Besetzt
call_wait_screen_dos_button_busy_descr=Zur Shell wechseln. Anrufer erhalten ein Besetztzeichen.
call_wait_screen_user_button_not_busy=Benutzer – Frei
call_wait_screen_user_button_not_busy_descr=Als normaler Benutzer anmelden. Der RING-Alarm wird aktiviert.
call_wait_screen_sysop_button_not_busy=Sysop – Frei
call_wait_screen_sysop_button_not_busy_descr=Als Sysop anmelden. Der RING-Alarm wird aktiviert.
call_wait_screen_dos_button_not_busy=Shell – Frei
call_wait_screen_dos_button_not_busy_descr=Zur Shell wechseln. Anrufer erhalten KEIN Besetztzeichen.
call_wait_screen_call_log_on=Anrufprotokoll – Ein
call_wait_screen_call_log_off=Anrufprotokoll – Aus
call_wait_screen_call_log_descr=Wenn eingeschaltet, werden Anrufe protokolliert.
call_wait_screen_page_bell_on=Rufsignal ein
call_wait_screen_page_bell_off=Rufsignal aus
call_wait_screen_page_bell_descr=Das System gibt einen Signalton aus, wenn ein Benutzer den Sysop ruft.
call_wait_screen_alarm_on=Alarm ein
call_wait_screen_alarm_off=Alarm aus
call_wait_screen_alarm_descr=Das System gibt bei der Anmeldung eines Benutzers usw. einen Signalton aus.
call_wait_screen_monitor_button_not_busy=ICBMoni
call_wait_screen_monitor_button_not_busy_descr=ICBMoni zur Überwachung der Node-Aktivität starten.
call_wait_screen_system_manager=ICBSM
call_wait_screen_system_manager_descr=IcyBoard System Manager zur Pflege der Benutzerdatei starten.
call_wait_screen_setup=ICBSetup
call_wait_screen_setup_descr=ICBSetup zur Änderung der IcyBoard-Konfiguration starten.
call_wait_screen_icb_text=ICBText
call_wait_screen_icb_text_descr=Die ICBText-Dateien des Systems bearbeiten.
call_wait_screen_total_statistics=GESAMT-Statistik
call_wait_screen_today_statistics=TAGES-Statistik
call_wait_screen_statistics_descr=Zwischen Gesamt- und Tagesstatistik wechseln.
call_wait_screen_show_statistics=Statistik anzeigen
call_wait_screen_show_statistics_descr=Alle Statistiken des Systems anzeigen.
call_wait_screen_sys_ready = System bereit für Anrufe
call_wait_screen_last_caller = Letzter Anrufer:
call_wait_screen_last_caller_none = Keiner
call_wait_screen_num_calls = Anrufe:
call_wait_screen_num_msgs = Nachr.:
call_wait_screen_num_dls = D/Ls:
call_wait_screen_num_uls = U/Ls:

# Systemstatistik
icb_system_statistics_title = [ IcyBoard-Systemstatistik ]
icb_system_statistics_footer = [ (↑), (↓), (Del) Zurücksetzen, (Esc) Ende ]
icb_system_statistics_confirm_reset = [ ALLE Statistiken samt Anrufernummer zurücksetzen? (Y) bestätigt ]
icb_system_statistics_header = Statistik
icb_system_statistics_total_calls = Anrufe gesamt
icb_system_statistics_total_messages = Nachrichten gesamt
icb_system_statistics_total_uploads = Uploads gesamt
icb_system_statistics_total_uploads_kb = Upload-KB gesamt
icb_system_statistics_total_downloads = Downloads gesamt
icb_system_statistics_total_downloads_kb = Download-KB gesamt
icb_system_statistics_today_calls = Anrufe heute
icb_system_statistics_today_messages = Nachrichten heute
icb_system_statistics_today_uploads = Uploads heute
icb_system_statistics_today_uploads_kb = Upload-KB heute
icb_system_statistics_today_downloads = Downloads heute
icb_system_statistics_today_downloads_kb = Download-KB heute

# Node-Überwachung
icbmoni_title = [ IcyBoard-Node-Überwachung ]
icbmoni_footer = [ (↑), (↓), (Esc) Ende ]
icbmoni_on_note_footer = [ (↑), (↓), (Return) Überwachen, (Esc) Ende ]
icbmoni_no_caller = Kein Anrufer auf diesem Node
icbmoni_user_log_in = Meldet sich an
icbmoni_user_browse_menu = Blättert in Menüs
icbmoni_user_enter_message = Schreibt eine Nachricht
icbmoni_comment_to_sysop = Schreibt an den Sysop
icbmoni_user_browse_files = Durchsucht Dateien
icbmoni_user_read_messages = Liest Nachrichten
icbmoni_user_read_bulletins = Liest Bulletins
icbmoni_user_take_survey = Beantwortet eine Umfrage
icbmoni_user_upload = Lädt Dateien hoch
icbmoni_user_download = Lädt Dateien herunter
icbmoni_user_logoff = Meldet sich ab
icbmoni_user_door = Führt ein Door aus
icbmoni_user_chat_with_sysop = Chat mit dem Sysop
icbmoni_user_group_chat = Gruppenchat
icbmoni_user_page_sysop = Ruft den Sysop
icbmoni_user_read_broadcast = Liest eine Rundnachricht
icbmoni_status_header = Status
icbmoni_user_header = Benutzer
icbmoni_protocol_header = Protokoll
icbmoni_log_in=Benutzer meldet sich an…
icbmoni_web_admin_url = Webverwaltung: { $url }
icbmoni_web_admin_token = Token: { $token }
quick_save=Schnell

# ICBSetup
icb_setup_key_main_help=↑ Auf  ↓ Ab  F1 Hilfe  ␛ Ende
icb_setup_key_menu_help=↑ Auf  ↓ Ab  F1 Hilfe  ␛ Zurück
icb_setup_key_menu_edit_help=↑ Auf  ↓ Ab  F1 Hilfe  F2 Datei bearbeiten  ␛ Zurück
icb_setup_key_menu_create_help=↑ Auf  ↓ Ab  F1 Hilfe  F3 Datei erstellen  ␛ Zurück
icb_setup_key_conf_list_help=↑ Auf  ↓ Ab  INS Neu  ␡ Löschen  PgUp/Dn Verschieben ␛ Zurück
icb_setup_main_title=Hauptmenü
icb_setup_main_use_label=Für ICB { $version }
icb_setup_main_sysop_info=Sysop-Informationen
icb_setup_main_sysop_info-help=
    # Sysop-Informationen

    Hier werden unter anderem Name, lokales Anmeldepasswort und die
    Grafikeinstellungen des Sysops festgelegt.

    Diese Angaben zum Sysop werden nicht in dessen Datensatz in der
    Benutzerdatei gespeichert.
icb_setup_main_file_locs=Dateipfade
icb_setup_main_file_locs-help=
    # Dateipfade

    Dieses Menü verteilt die Systempfade und Dateinamen von IcyBoard
    auf mehrere Eingabemasken.
icb_setup_main_con_info=Verbindungen
icb_setup_main_con_info-help=
    # Verbindungen

    Dieses Menü bietet mehrere Masken mit Einstellungen dazu,
    wie Benutzer eine Verbindung zu IcyBoard herstellen.
icb_setup_main_board_cfg=Board-Konfiguration
icb_setup_main_board_cfg-help=
    # Board-Konfiguration

    Hier werden Angaben zum Board selbst gespeichert.
icb_setup_main_evt_setup=Ereignisse
icb_setup_main_evt_setup-help=
    # Ereignisse

    Hier werden allgemeine Angaben zur Ausführung von Ereignissen gemacht.
    F2 auf der Datei EVENT.DAT öffnet die vollständige Ereigniskonfiguration.
icb_setup_main_subscription=Abonnements
icb_setup_main_subscription-help=
    # Abonnements

    Hier wird das Abonnementsystem eingerichtet: Neue Benutzer erhalten
    ein Abonnement für eine festgelegte Anzahl von Tagen. Danach werden
    ihre Sicherheitsstufen entsprechend den Vorgaben geändert.
icb_setup_main_conf_opt=Konfigurationsoptionen
icb_setup_main_conf_opt-help=
    # Konfigurationsoptionen

    Dieses Untermenü führt zu mehreren Masken, mit denen sich das
    Verhalten eines IcyBoard-Systems anpassen lässt.
icb_setup_main_sec_levels=Sicherheitsstufen
icb_setup_main_sec_levels-help=
    # Sicherheitsstufen

    Dieses Menü bietet weitere Auswahlmöglichkeiten zur Festlegung
    der Sicherheitsstufen für Sysop-Funktionen, Sysop-Befehle und
    Benutzerbefehle.
icbsm_define_editors=Text- und Grafikeditoren
icbsm_customize_colors=Farben anpassen
icbsm_text_editor=Texteditor
icbsm_graphics_editor=Grafikeditor
icbsm_color_title=Farbanpassung
icbsm_color_default_1=Standardfarbsatz #1
icbsm_color_default_2=Standardfarbsatz #2
icbsm_color_bw=Standard-S/W-Farben
icbsm_color_customize=Farben anpassen
icb_setup_main_acc_cfg=Abrechnung
icb_setup_main_acc_cfg-help=
    # Abrechnung

    Die Abrechnung ist ein optionaler Bestandteil eines IcyBoard-Systems.

    In diesen Masken lassen sich Kosten oder Vergütungen für verschiedene
    Aktivitäten im BBS sowie Hauptnutzungszeiten und Feiertage festlegen.
icb_setup_main_new_user=Neue Benutzer
icb_setup_main_new_user-help=
    # Neue Benutzer

    Dieses Menü legt Vorgaben und Fragen für neue Benutzer fest.

    Dazu gehören die anfängliche Sicherheitsstufe, die Standardgruppen
    und die Fragen, die neuen Benutzern gestellt werden.
icb_setup_msg_networking=Nachrichten und Netze
icb_setup_msg_networking-help=
    # Nachrichten und Netze

    Dieses Menü dient zur Einrichtung von QWK, FTN usw. für IcyBoard.
icb_setup_mb_conf=Hauptkonferenz
icb_setup_mb_conf-help=
    # Hauptkonferenz

    Hier werden die für die Hauptkonferenz erforderlichen Angaben
    festgelegt, einschließlich Downloadpfaden, Bulletins, Skripten und Menüs.
icb_setup_conferences=Konferenzen
icb_setup_conferences-help=
    # Konferenzen

    Zeigt eine Liste der Konferenzen, die zur Bearbeitung ausgewählt
    werden können.

    Konferenzen lassen sich hinzufügen, löschen, umordnen und bearbeiten.
board_config_title=Board-Konfiguration
board_name=Boardname
board_name-status=Der Boardname wird dem Anrufer bei der Anmeldung angezeigt.
board_name-help=
    # Boardname

    Hier den Namen des BBS eingeben. Dieser Name wird dem Anrufer
    beim Verbindungsaufbau angezeigt.
allow_iemsi=IEMSI erlauben
allow_iemsi-status=IEMSI-Anmeldung erlauben
allow_iemsi-help=
    # IEMSI erlauben

    IEMSI ist ein Verfahren zur automatischen Anmeldung und zum Austausch
    von Fähigkeiten für fortgeschrittene BBS-Clients. Der Client sendet
    Name, Terminalfunktionen (ANSI/Farbe, Größe) und bevorzugte Protokolle.
    So kann IcyBoard die meisten Abfragen überspringen und die Ausgabe sofort anpassen.

    Treffen keine IEMSI-Daten ein, erfolgt die normale interaktive Anmeldung.
    Da klassisches IEMSI unverschlüsselt ist, sollte es nur über
    vertrauenswürdige oder abgesicherte Verbindungen verwendet werden.
board_iemsi_location=Standort
board_iemsi_location-status=Über IEMSI übermittelter Boardstandort
board_iemsi_location-help=
    # Standort

    Lesbare Standortangabe (Stadt/Region), die beim IEMSI-Handshake übermittelt wird.
board_iemsi_operator=Betreiber
board_iemsi_operator-status=Über IEMSI übermittelter Boardbetreiber
board_iemsi_operator-help=
    # Betreiber

    Name oder Pseudonym des Boardbetreibers/Sysops, der an den Client gesendet wird.
board_iemsi_notice=Hinweis
board_iemsi_notice-status=Über IEMSI übermittelter Boardhinweis
board_iemsi_notice-help=
    # Hinweis

    Kurzer Begrüßungs- oder Statustext, den geeignete IEMSI-Clients nach der Anmeldung anzeigen.
board_iemsi_caps=Fähigkeiten
board_iemsi_caps-status=Über IEMSI übermittelte Boardfähigkeiten
board_iemsi_caps-help=
    # Fähigkeiten

    Zeichenfolge mit Kennungen unterstützter Funktionen (z. B. ANSI,COLOR,RIP,DOORS,MAIL).
    Wird beim IEMSI-Handshake verwendet.
board_node_num=Anzahl Nodes
board_node_num-status=Maximale Anzahl gleichzeitig aktiver Nodes
board_node_num-help=
    # Nodes

    Maximal erlaubte Anzahl von Nodes. Begrenzt DDoS-Angriffe.
who_include_city=Ort in WHO anzeigen
who_include_city-status=Ortsfeld in der WHO-Anzeige ausgeben
who_include_city-help=
    # Ort in WHO anzeigen

    Wenn ein Benutzer WHO an der IcyBoard-Eingabeaufforderung eingibt,
    bestimmt diese Einstellung, ob das Ortsfeld jedes angemeldeten
    Benutzers in der Liste erscheint.
web_admin_enabled=Webverwaltung aktivieren
web_admin_enabled-status=Webverwaltungsserver zusammen mit IcyBoard starten
web_admin_enabled-help=
    # Webverwaltung aktivieren

    Startet die Weboberfläche zur Verwaltung während des IcyBoard-Betriebs.
    Sie ist standardmäßig deaktiviert und ersetzt weder icbsetup noch icbsm.
web_admin_address=Webverwaltungsadresse
web_admin_address-status=Netzwerkadresse des Webverwaltungsservers
web_admin_address-help=
    # Webverwaltungsadresse

    Netzwerkadresse, an der der Webverwaltungsserver Verbindungen annimmt.
    Bei 127.0.0.1 belassen, sofern Fernzugriff nicht ausdrücklich benötigt wird.
web_admin_port=Webverwaltungsport
web_admin_port-status=TCP-Port des Webverwaltungsservers
web_admin_port-help=
    # Webverwaltungsport

    TCP-Port, an dem der Webverwaltungsserver Verbindungen annimmt.
    Der Standardport ist 8787.
web_admin_allow_remote=Web-Fernverwaltung erlauben
web_admin_allow_remote-status=Webverwaltung außerhalb von localhost erlauben
web_admin_allow_remote-help=
    # Web-Fernverwaltung erlauben

    Erlaubt dem Verwaltungsserver die Bindung an eine Nicht-Loopback-Adresse.
    Dadurch wird die Boardverwaltung im Netzwerk erreichbar. Nur hinter
    einem authentifizierten TLS-Reverse-Proxy und mit einem starken Zugriffstoken aktivieren.
who_show_alias=Alias in WHO anzeigen
who_show_alias-status=Alias statt Name in der WHO-Anzeige verwenden
who_show_alias-help=
    # Alias in WHO anzeigen

    Zeigt in der WHO-Anzeige den Alias anstelle des Namens an.
date_format=Datumsformat
date_format-status=Im System verwendetes Datumsformat
date_format-help=
    # Datumsformat

    Das von IcyBoard standardmäßig verwendete Datumsformat.
new_user_options_title=Neue Benutzer
new_user_options_ask_label=Neue Benutzer fragen nach:
new_user_security_level=Sicherheitsstufe
new_user_security_level-status=Anfängliche Sicherheitsstufe neuer Benutzer
new_user_security_level-help=
    # Sicherheitsstufe

    Diese Sicherheitsstufe erhält ein Benutzer bei der Registrierung.
    Sie bestimmt den Zugriff auf Befehle, Konferenzen und Dateibereiche.
    Stufe 0 sperrt den Benutzer vom Board aus.
allow_one_name_users=Einzelne Namen erlauben
allow_one_name_users-status=Benutzer mit nur einem Namen erlauben
allow_one_name_users-help=
    # Einzelne Namen erlauben

    Erlaubt die Registrierung mit einem einzelnen Namen statt Vor- und Nachname.
    Ausschalten, wenn die Benutzerliste nur vollständige echte Namen enthalten soll.
auto_register_conferences=In öffentlichen Konferenzen registrieren
auto_register_conferences-status=Neue Benutzer werden in allen öffentlichen Konferenzen registriert.
auto_register_conferences-help=Konferenzen mit eigenen Sicherheitsanforderungen bleiben ausgenommen.
new_user_groups=Standardgruppen
new_user_groups-status=Standardgruppen für neue Benutzer
new_user_groups-help=
    # Standardgruppen

    Durch Kommas getrennte Gruppen, denen neue Benutzer zugeordnet werden.
    Sicherheitsausdrücke in Menüs, Konferenzen und Dateibereichen prüfen die
    Gruppenzugehörigkeit. Damit erhalten neue Benutzer Rechte, die nicht
    allein an eine Sicherheitsstufe gebunden sind.
ask_city_or_state=Stadt oder Region
ask_city_or_state-status=Nach Stadt oder Region fragen
ask_city_or_state-help=
    # Stadt oder Region

    Fragt neue Benutzer nach ihrem Wohnort. Spätere Änderungen sind mit W möglich.
    Die Antwort erscheint in WHO, wenn das Board das Ortsfeld anzeigen soll.
ask_address=Adresse
ask_address-status=Nach der Adresse fragen
ask_address-help=
    # Adresse

    Fragt neue Benutzer nach der Postanschrift. Spätere Änderungen sind mit W möglich.
    Nur einschalten, wenn das Board die Adresse wirklich benötigt,
    beispielsweise für ein postalisch abgewickeltes Abonnement.
ask_verification=Identitätsprüfung
ask_verification-status=Nach einem Merkmal zur Identitätsprüfung fragen
ask_verification-help=
    # Identitätsprüfung

    Fragt nach einer nur dem Benutzer bekannten Antwort, etwa dem Geburtsnamen
    der Mutter. Der Sysop kann später erneut danach fragen, um bei der
    Wiederherstellung eines Kontos die Identität des Benutzers zu prüfen.
ask_bus_data_phone=Geschäfts-/Datentelefon
ask_bus_data_phone-status=Nach Geschäfts- oder Datentelefonnummer fragen
ask_bus_data_phone-help=
    # Geschäfts-/Datentelefon

    Fragt neue Benutzer nach einer Geschäfts- oder Datentelefonnummer.
    Spätere Änderungen sind mit W möglich.
ask_home_phone=Privattelefon
ask_home_phone-status=Nach der privaten Telefonnummer fragen
ask_home_phone-help=
    # Privattelefon

    Fragt neue Benutzer nach einer privaten oder Sprachtelefonnummer.
    Spätere Änderungen sind mit W möglich.
ask_comment=Kommentar
ask_comment-status=Nach einem Kommentar fragen
ask_comment-help=
    # Kommentar

    Fragt neue Benutzer nach einer Zeile über sich selbst. Sie wird im
    Benutzerdatensatz gespeichert und dem Sysop im Benutzereditor angezeigt.
ask_clr_msg=Bildschirm löschen
ask_clr_msg-status=Nach dem Löschen zwischen Nachrichten fragen
ask_clr_msg-help=
    # Bildschirm löschen

    Fragt, ob der Bildschirm zwischen Nachrichten gelöscht werden soll.
    Benutzer langsamer Terminals oder mit Rückblätterfunktion wählen meist Nein.
ask_fse=Vollbildeditor
ask_fse-status=Nach dem Vollbildeditor fragen
ask_fse-help=
    # Vollbildeditor

    Fragt neue Benutzer, ob sie Nachrichten mit dem Vollbildeditor statt
    mit dem Zeileneditor schreiben möchten.
ask_xfer_protocol=Protokolle
ask_xfer_protocol-status=Nach dem Übertragungsprotokoll fragen
ask_xfer_protocol-help=
    # Protokolle

    Fragt nach dem Standard-Übertragungsprotokoll aus der Protokollliste.
    Ohne diese Abfrage beginnen alle mit dem Boardstandard und können
    später mit T ein anderes Protokoll wählen.
ask_date_format=Datumsformat
ask_date_format-status=Nach dem Datumsformat fragen
ask_date_format-help=
    # Datumsformat

    Fragt nach dem gewünschten Datumsformat aus den angebotenen Formaten.
    Ohne diese Abfrage beginnen alle Benutzer mit dem Boardstandard.
ask_alias=Alias
ask_alias-status=Nach einem Alias fragen
ask_alias-help=
    # Alias

    Fragt nach einem Pseudonym zur Anzeige. Ein Alias wird nur in Konferenzen
    verwendet, die Aliasse erlauben. Ob er später geändert werden darf,
    legt „Aliasänderung erlauben“ fest.
ask_gender=Geschlecht
ask_gender-status=Nach dem Geschlecht fragen
ask_gender-help=
    # Geschlecht

    Fragt nach dem Geschlecht. Die Angabe wird im Benutzerdatensatz
    gespeichert und kann von PPE-Programmen gelesen werden.
ask_birthdate=Geburtsdatum
ask_birthdate-status=Nach dem Geburtsdatum fragen
ask_birthdate-help=
    # Geburtsdatum

    Fragt nach dem Geburtsdatum und speichert es im Benutzerdatensatz.
    PPE-Programme können es für Altersprüfungen oder Geburtstagsgrüße lesen.
ask_email=E-Mail
ask_email-status=Nach der E-Mail-Adresse fragen
ask_email-help=
    # E-Mail

    Fragt neue Benutzer nach einer E-Mail-Adresse.
    Spätere Änderungen sind mit W möglich.
ask_web_address=Webadresse
ask_web_address-status=Nach der Webadresse fragen
ask_web_address-help=
    # Webadresse

    Fragt neue Benutzer nach der Adresse ihrer Homepage.
    Spätere Änderungen sind mit W möglich.
ask_use_short_descr=Kurzbeschreibung
ask_use_short_descr-status=Nach kurzen Dateibeschreibungen fragen
ask_use_short_descr-help=
    # Kurzbeschreibung

    Fragt, ob Dateilisten nur die erste Zeile jeder Beschreibung zeigen sollen.
    Das hält Listen auf kleinen Bildschirmen kurz; die Einstellung kann
    später mit W geändert werden.
subscription_information_title=Abonnements
subscription_is_enabled=Abonnements aktivieren
subscription_is_enabled-status=Ablauf von Benutzerkonten berücksichtigen.
subscription_is_enabled-help=
    # Abonnements aktivieren

    Legt fest, ob das Board bei der Anmeldung das Ablaufdatum im
    Benutzerdatensatz prüft. Ausgeschaltet wird es unabhängig vom Alter
    ignoriert. Eingeschaltet erhalten Benutzer nach Ablauf die
    Sicherheitsstufe für abgelaufene Konten.
subscription_length=Standardlaufzeit in Tagen
subscription_length-status=Anzahl der Tage bis zum Ablauf des Kontos.
subscription_length-help=
    # Standardlaufzeit in Tagen

    Laufzeit eines neuen Abonnements ab dem Registrierungstag.
    365 entspricht einem Jahr; 0 lässt das Ablaufdatum leer,
    sodass das Konto nie abläuft.
default_expired_level=Stufe nach Ablauf
default_expired_level-status=Sicherheitsstufe für abgelaufene Konten.
default_expired_level-help=
    # Stufe nach Ablauf

    Auf diese Sicherheitsstufe fällt ein Benutzer nach Ablauf zurück.
    Anfangs entspricht sie der Stufe für neue Benutzer. Niedriger setzen,
    um abgelaufenen Konten weniger Rechte als zahlenden zu geben,
    oder auf 0 setzen, um sie zu sperren.
warning_days=Warntage vor Ablauf
warning_days-status=Zeigt vor Ablauf des Kontos die Datei WARNING an.
warning_days-help=
    # Warntage vor Ablauf

    So viele Tage vor Ablauf wird bei der Anmeldung die Datei WARNING
    angezeigt, damit Zeit zur Verlängerung bleibt. 0 schaltet die Warnung aus.
sysop_information_title=Sysop-Informationen
sysop_name=Name des Sysops
sysop_name-status=Wenn NICHT der echte Name verwendet wird.
sysop_name-help=
    # Name des Sysops

    Hier den Vornamen des Sysops eingeben.

    HINWEIS: NICHT den vollständigen Namen verwenden.
    Hier gehört nur der Vorname hin.
    Der VOLLSTÄNDIGE NAME gehört mit `icbsm` in Datensatz #1 der Datei USERS.
local_password=Lokales Passwort
local_password-status=Passwort für den Anruf-Warteschirm.
local_password-help=
    # Lokales Passwort

    Passwort, das der Sysop an der LOKALEN Station eingibt,
    um vom Anruf-Warteschirm in IcyBoard zu gelangen.
require_password_to_exit=Passwort zum Beenden
require_password_to_exit-status=Passwort zum Verlassen des Anruf-Warteschirms verlangen.
require_password_to_exit-help=
    # Passwort zum Beenden

    Wenn aktiviert, muss der Sysop das lokale Passwort eingeben,
    um den Anruf-Warteschirm zu verlassen.
sys_info_external_editor=Externer Editor
sys_info_external_editor-status=Externer Editor für Sysop-Nachrichten.
sys_info_external_editor-help=
    # Externer Editor

    Name des externen Editors, mit dem der Sysop allgemeine Textdateien
    aus `icbsetup` heraus bearbeitet.
sys_info_graphics_editor=Grafikeditor
sys_info_graphics_editor-status=Editor für ANSI- und Grafikdateien.
sys_info_graphics_editor-help=
    # Grafikeditor

    Name des Editors für ANSI- und Grafikdateien.
sys_info_theme=Farbschema
sys_info_theme-status=Farbschema
sys_info_theme-help=
    # Farbschema

    Das von `icbsetup` verwendete Farbschema.
use_real_name=Echten Namen verwenden
use_real_name-status=Echten Namen des Sysops verwenden.
use_real_name-help=
    # Echten Namen verwenden

    Bei 'N' werden Nachrichten des Sysops unter dem Absender SYSOP gespeichert.

    Bei 'Y' wird der Name aus Datensatz #1 der Benutzerdatei verwendet.
sec_level_menu_title=Sicherheitsstufen
sec_level_menu_sysop_funcs=Sysop-Funktionen
sec_level_menu_sysop_commands=Sysop-Befehle
sec_level_menu_user_commands=Benutzerbefehle
sysop_commands_title=Sysop-Befehle
sysop_sec_level=Sysop-Stufe
sysop_sec_level-status=Für Sysop-Menü und temporäre Hochstufung mit F1
sysop_sec_level-help=
    # Sysop-Stufe

    Ab dieser Stufe gilt ein Benutzer als Sysop und sieht das Sysop-Menü
    statt des Benutzermenüs. Eine temporäre Hochstufung oder die Kennzeichnung
    als Konferenz-Sysop vergibt diese Stufe. Sie gewährt nicht automatisch
    die folgenden einzelnen Sysop-Befehle; diese behalten eigene Stufen.
sysop_sec_read_all_comments=Alle Kommentare lesen
sysop_sec_read_all_comments-status=Stufe zum Lesen aller Kommentare
sysop_sec_read_all_comments-help=
    # Alle Kommentare lesen

    Mit C verfasste Kommentare sind die vertraulichsten Nachrichten des Boards.
    Wer sie lesen darf, darf daher auch alle anderen Nachrichten lesen.
sysop_sec_read_all_mail=Alle Nachrichten außer Kommentaren
sysop_sec_read_all_mail-status=Stufe zum Lesen aller Nachrichten außer Kommentaren
sysop_sec_read_all_mail-help=
    # Alle Nachrichten lesen

    Erlaubt das Lesen privater Nachrichten, deren Absender und Empfänger
    andere Benutzer sind. Kommentare an den Sysop bleiben durch die obige Stufe geschützt.
sysop_sec_copy_move_messages=Nachrichten kopieren/verschieben
sysop_sec_copy_move_messages-status=Stufe zum Kopieren oder Verschieben zwischen Bereichen
sysop_sec_copy_move_messages-help=
    # Nachrichten kopieren oder verschieben

    Erlaubt am Nachrichtenende das Kopieren oder Verschieben einer
    Nachricht in eine andere Konferenz.
sysop_sec_enter_color_codes_in_messages=@-Variablen in Nachrichten
sysop_sec_enter_color_codes_in_messages-status=Stufe zum Einfügen von @-Variablen in Nachrichten
sysop_sec_enter_color_codes_in_messages-help=
    # @-Variablen in Nachrichten

    Erlaubt @-Makros wie @USER@ oder @MORE@ im Nachrichtentext,
    die beim Lesen ausgewertet werden. @X-Farbcodes sind für alle erlaubt.
sysop_sec_edit_any_message=Beliebige Nachricht bearbeiten
sysop_sec_edit_any_message-status=Stufe zum Bearbeiten beliebiger Nachrichten
sysop_sec_edit_any_message-help=
    # Beliebige Nachricht bearbeiten

    Erlaubt das Bearbeiten aller lesbaren Nachrichten, nicht nur eigener.
    Höher als die Stufe für eigene Nachrichten setzen: Wer dieses Recht
    besitzt, kann anderen Benutzern Aussagen unterschieben.
sysop_sec_not_update_msg_read=Lesestatus nicht aktualisieren
sysop_sec_not_update_msg_read-status=Befehl R O
sysop_sec_not_update_msg_read-help=
    # Lesestatus nicht aktualisieren

    Lesen mit R O lässt den Zeiger auf die zuletzt gelesene Nachricht
    unverändert. Benutzer können damit Nachrichten lesen, ohne dass das Board dies vermerkt.
sysop_sec_use_broadcast_command=BROADCAST verwenden
sysop_sec_use_broadcast_command-status=Befehl BR
sysop_sec_use_broadcast_command-help=
    # BROADCAST verwenden

    Erlaubt mit BR eine einzeilige Nachricht an einen Benutzer auf einem
    anderen Node oder gleichzeitig an alle Nodes zu senden.
sysop_sec_view_private_uploads=Private Uploads anzeigen
sysop_sec_view_private_uploads-status=Stufe zum Anzeigen des privaten Uploadverzeichnisses
sysop_sec_view_private_uploads-help=
    # Private Uploads anzeigen

    Erlaubt das Auflisten des privaten Uploadverzeichnisses der aktuellen
    Konferenz. Dort warten neue Uploads auf das Verschieben in ein öffentliches Verzeichnis.
sysop_sec_enter_generic_messages=Allgemeine Nachrichten schreiben
sysop_sec_enter_generic_messages-status=Nachrichten an @USER@
sysop_sec_enter_generic_messages-help=
    # Allgemeine Nachrichten schreiben

    Erlaubt Nachrichten an @USER@ oder einen Sicherheitsstufenbereich,
    sodass eine Nachricht viele Benutzer erreicht und jeden persönlich anspricht.
sysop_sec_edit_message_headers=Nachrichtenköpfe bearbeiten
sysop_sec_edit_message_headers-status=Stufe zum Bearbeiten von Nachrichtenköpfen
sysop_sec_edit_message_headers-help=
    # Nachrichtenköpfe bearbeiten

    Erlaubt das Ändern von Absender, Empfänger und Schutz einer Nachricht.
sysop_sec_protect_unprotect_messages=Nachrichtenschutz ändern
sysop_sec_protect_unprotect_messages-status=Stufe zum Schützen/Freigeben einer Nachricht
sysop_sec_protect_unprotect_messages-help=
    # Nachrichtenschutz ändern

    Erlaubt am Nachrichtenende, eine Nachricht privat oder wieder öffentlich zu machen.
sysop_sec_overwrite_files_on_uploads=Dateien beim Upload überschreiben
sysop_sec_overwrite_files_on_uploads-status=Stufe zum Überschreiben vorhandener Dateien beim Upload
sysop_sec_overwrite_files_on_uploads-help=
    # Dateien beim Upload überschreiben

    Ist eine Datei bereits vorhanden, kann der Benutzer sie ersetzen,
    beide Dateien behalten oder den Upload abbrechen, statt nur abgewiesen zu werden.
sysop_sec_set_pack_out_date_on_messages=Löschdatum setzen
sysop_sec_set_pack_out_date_on_messages-status=Stufe zum Setzen des Löschdatums einer Nachricht
sysop_sec_set_pack_out_date_on_messages-help=
    # Löschdatum setzen

    Erlaubt ein Datum festzulegen, an dem die Nachricht automatisch entfernt wird.
    Das eignet sich für Ankündigungen mit einem bekannten Ende ihrer Gültigkeit.
sysop_sec_see_all_return_receipts=Alle Lesebestätigungen ansehen
sysop_sec_see_all_return_receipts-status=Stufe zum Anzeigen aller Lesebestätigungen
sysop_sec_see_all_return_receipts-help=
    # Alle Lesebestätigungen ansehen

    Eine Lesebestätigung sieht normalerweise nur der Benutzer, der sie angefordert hat.
    Diese Stufe erlaubt auch das Lesen fremder Bestätigungen.
sysop_functions_title=Sysop-Funktionen
sysop_sec_1_view_caller_log=(1) Anrufprotokoll ansehen/drucken
sysop_sec_1_view_caller_log-status=Anrufprotokoll ansehen/drucken
sysop_sec_1_view_caller_log-help=
    # (1) Anrufprotokoll ansehen

    Erlaubt das Lesen des Protokolls, das Anrufer und ihre Aktivitäten erfasst.
    Sysop-Stufen liegen üblicherweise bei 100 oder höher.
sysop_sec_2_view_usr_list=(2) Benutzerliste ansehen/drucken
sysop_sec_2_view_usr_list-status=Benutzerliste ansehen/drucken
sysop_sec_2_view_usr_list-help=
    # (2) Benutzerliste ansehen

    Listet mit dem Sysop-Befehl die Benutzerdatei auf und zeigt dabei
    mehr Angaben als die für Benutzer zugängliche Suche.
sysop_sec_3_pack_renumber_msg=(3) Nachrichten packen/nummerieren
sysop_sec_3_pack_renumber_msg-status=Nachrichten packen und neu nummerieren
sysop_sec_3_pack_renumber_msg-help=
    # (3) Nachrichten packen/neu nummerieren

    Packt eine Nachrichtenbasis, entfernt gelöschte Nachrichten und vergibt
    neue Nummern. Die Basis wird neu geschrieben; daher eine hohe Stufe wählen.
sysop_sec_4_recover_deleted_msg=(4) Nachricht wiederherstellen
sysop_sec_4_recover_deleted_msg-status=Gelöschte Nachricht wiederherstellen
sysop_sec_4_recover_deleted_msg-help=
    # (4) Gelöschte Nachricht wiederherstellen

    Stellt eine gelöschte Nachricht wieder her, solange die Basis seit
    der Löschung noch nicht gepackt wurde.
sysop_sec_5_list_message_hdr=(5) Nachrichtenköpfe auflisten
sysop_sec_5_list_message_hdr-status=Nachrichtenköpfe auflisten
sysop_sec_5_list_message_hdr-help=
    # (5) Nachrichtenköpfe auflisten

    Listet nur die Nachrichtenköpfe auf, um eine Basis schnell zu überblicken,
    ohne die Nachrichten selbst lesen zu müssen.
sysop_sec_6_view_any_file=(6) Beliebige Datei ansehen
sysop_sec_6_view_any_file-status=Beliebige Datei ansehen
sysop_sec_6_view_any_file-help=
    # (6) Beliebige Datei ansehen

    Erlaubt das Anzeigen jeder Datei im System, unabhängig von ihrem Ort
    und davon, ob der Benutzer das zugehörige Dateiverzeichnis auflisten darf.
sysop_sec_7_user_maint=(7) Benutzerverwaltung
sysop_sec_7_user_maint-status=Benutzerverwaltung
sysop_sec_7_user_maint-help=
    # (7) Benutzerverwaltung

    Erlaubt das Bearbeiten von Benutzerdatensätzen direkt im Board:
    Sicherheitsstufe, Kennzeichen, Ablaufdatum und übrige Angaben.
sysop_sec_8_pack_usr_file=(8) Benutzerdatei packen
sysop_sec_8_pack_usr_file-status=Benutzerdatei packen
sysop_sec_8_pack_usr_file-help=
    # (8) Benutzerdatei packen

    Entfernt Datensätze, die den angegebenen Kriterien entsprechen,
    und schreibt die Benutzerdatei neu.
sysop_sec_9_exit_to_dos=(9) Fernwechsel zur Shell
sysop_sec_9_exit_to_dos-status=Aus der Ferne zur Shell wechseln
sysop_sec_9_exit_to_dos-help=
    # (9) Zur Shell wechseln

    In PCBoard war dies die Stufe zum Fernwechsel vom Board zu DOS.
sysop_sec_10_shelled_dos_func=(10) PPE ausführen
sysop_sec_10_shelled_dos_func-status=Stufe zum Ausführen eines PPE an der Eingabeaufforderung
sysop_sec_10_shelled_dos_func-help=PCBoard verwendete diese Stufe für Befehl 10 und seine DOS-Shell. IcyBoard hat keine DOS-Shell; die Stufe schützt stattdessen den PPE-Befehl.
sysop_sec_11_view_other_nodes=(11) Andere Nodes ansehen
sysop_sec_11_view_other_nodes-status=Andere Nodes ansehen
sysop_sec_11_view_other_nodes-help=
    # (11) Andere Nodes ansehen

    Zeigt an, wer auf anderen Nodes angemeldet ist.
sysop_sec_12_logoff_alt_node=(12) Anderen Node abmelden
sysop_sec_12_logoff_alt_node-status=Benutzer auf anderem Node abmelden
sysop_sec_12_logoff_alt_node-help=
    # (12) Anderen Node abmelden

    Meldet einen Benutzer ab, der auf einem anderen Node angemeldet ist.
sysop_sec_13_view_alt_node_callers=(13) Anrufer anderer Nodes
sysop_sec_13_view_alt_node_callers-status=Anrufer anderer Nodes ansehen
sysop_sec_13_view_alt_node_callers-help=
    # (13) Anrufer anderer Nodes ansehen

    Erlaubt das Lesen des Anrufprotokolls eines anderen Nodes statt nur des eigenen.
sysop_sec_14_drop_alt_node_to_dos=(14) Anderen Node zu DOS schicken
sysop_sec_14_drop_alt_node_to_dos-status=Anderen Node zur Shell schicken
sysop_sec_14_drop_alt_node_to_dos-help=
    # (14) Anderen Node zur Shell schicken

    In PCBoard war dies die Stufe, um einen anderen Node zum Wechsel zu DOS zu zwingen.
user_commands_title=Benutzerbefehle
user_sec_cmd_a=A) Konferenz verlassen
user_sec_cmd_a-status=Stufe zum Verlassen einer Konferenz
user_sec_cmd_a-help=
    # A) Konferenz verlassen

    Verlässt die aktuelle Konferenz und kehrt zur Hauptkonferenz zurück.
    Automatisch beigetretene Benutzer ohne dieses Recht sitzen fest;
    deshalb sollte dieser Befehl für alle erreichbar bleiben.
user_sec_cmd_b=B) Bulletins auflisten
user_sec_cmd_b-status=Stufe zum Lesen von Bulletins
user_sec_cmd_b-help=
    # B) Bulletins auflisten

    Listet die Bulletins der aktuellen Konferenz auf und erlaubt das Lesen.
user_sec_cmd_c=C) Kommentar an Sysop
user_sec_cmd_c-status=Stufe für Kommentare an den Sysop
user_sec_cmd_c-help=
    # C) Kommentar an Sysop

    Hinterlässt einen Kommentar für den Sysop. Es handelt sich um eine normale,
    an den Sysop adressierte Nachricht, geschützt durch die Stufe zum Lesen aller Kommentare.
user_sec_cmd_d=D) Datei herunterladen
user_sec_cmd_d-status=Stufe für Downloads
user_sec_cmd_d-help=
    # D) Datei herunterladen

    Erlaubt Downloads und das Markieren von Dateien dafür. Für mehrere Dateien
    gleichzeitig ist zusätzlich die Stapelübertragungsstufe nötig; auch das
    Dateiverzeichnis kann eine eigene Stufe verlangen.
user_sec_cmd_e=E) Nachricht schreiben
user_sec_cmd_e-status=Stufe zum Schreiben von Nachrichten
user_sec_cmd_e-help=
    # E) Nachricht schreiben

    Erlaubt das Schreiben von Nachrichten. Eine Konferenz kann selbst eine
    höhere Stufe verlangen; eine schreibgeschützte Konferenz lehnt unabhängig davon Nachrichten ab.
user_sec_cmd_f=F) Dateiverzeichnisse
user_sec_cmd_f-status=Stufe zum Auflisten von Dateiverzeichnissen
user_sec_cmd_f-help=
    # F) Dateiverzeichnisse

    Listet die Dateiverzeichnisse der aktuellen Konferenz auf.
    Einzelne Verzeichnisse können weiterhin eigene Zugriffsbeschränkungen haben.
user_sec_cmd_h=H) Hilfefunktionen
user_sec_cmd_h-status=Stufe zum Lesen von Hilfedateien
user_sec_cmd_h-help=
    # H) Hilfefunktionen

    Erlaubt das Lesen der Hilfe. Niedrig genug setzen, damit ganz neue
    Benutzer die Hilfe schon lesen können, bevor sie weitere Rechte erhalten.
user_sec_cmd_i=I) Begrüßung anzeigen
user_sec_cmd_i-status=Stufe zum erneuten Anzeigen der Begrüßung
user_sec_cmd_i-help=
    # I) Begrüßung anzeigen

    Zeigt nach der Anmeldung erneut den Begrüßungsbildschirm an.
user_sec_cmd_j=J) Konferenz betreten
user_sec_cmd_j-status=Stufe zum Betreten von Konferenzen
user_sec_cmd_j-help=
    # J) Konferenz betreten

    Erlaubt den Wechsel in eine andere Konferenz. Private Konferenzen verlangen
    zusätzlich eine Registrierung; jede Konferenz kann eine eigene Stufe verlangen.
user_sec_cmd_k=K) Nachricht löschen
user_sec_cmd_k-status=Stufe zum Löschen von Nachrichten
user_sec_cmd_k-help=
    # K) Nachricht löschen

    Erlaubt das Löschen einer Nachricht. Nicht lesbare Nachrichten können
    auch nicht gelöscht werden; Benutzer dürfen nur eigene Nachrichten löschen.
user_sec_cmd_l=L) Dateinamen suchen
user_sec_cmd_l-status=Stufe zur Suche nach Dateinamen
user_sec_cmd_l-help=
    # L) Dateinamen suchen

    Durchsucht alle für den Benutzer auflistbaren Dateiverzeichnisse
    nach einem Namen oder einem Muster mit Platzhaltern.
user_sec_cmd_m=M) Grafikmodus
user_sec_cmd_m-status=Stufe zum Wechsel des Grafikmodus
user_sec_cmd_m-help=
    # M) Grafikmodus

    Wechselt zwischen reinem Text und Grafik, einschließlich der
    Befehlsvarianten M CTTY und M ANSI.
user_sec_cmd_n=N) Neue Dateien suchen
user_sec_cmd_n-status=Stufe zur Suche nach neuen Dateien
user_sec_cmd_n-help=
    # N) Neue Dateien suchen

    Durchsucht die Dateiverzeichnisse nach Uploads seit der letzten Suche des Benutzers.
user_sec_cmd_o=O) Sysop rufen
user_sec_cmd_o-status=Stufe zum Rufen des Sysops
user_sec_cmd_o-help=
    # O) Sysop rufen

    Ruft den Sysop zum Chat. Ein Signal ertönt nur bei eingeschaltetem
    Rufsignal und innerhalb der Bereitschaftszeiten des Sysops.
user_sec_cmd_p=P) Seitenlänge
user_sec_cmd_p-status=Stufe zum Einstellen der Seitenlänge
user_sec_cmd_p-help=
    # P) Seitenlänge

    Legt fest, nach wie vielen ausgegebenen Zeilen das Board pausiert.

user_sec_cmd_q=Q) Nachrichtenübersicht
user_sec_cmd_q-status=Stufe zum Durchsehen der Nachrichtenköpfe
user_sec_cmd_q-help=
    # Q) Nachrichtenübersicht

    Zeigt Nachrichtenköpfe, ohne die Nachrichten zu lesen. Üblicherweise
    gilt dieselbe Stufe wie für das Lesen von Nachrichten.
user_sec_cmd_r=R) Nachrichten lesen
user_sec_cmd_r-status=Stufe zum Lesen von Nachrichten
user_sec_cmd_r-help=
    # R) Nachrichten lesen

    Erlaubt das Lesen von Nachrichten. Private Nachrichten zwischen anderen
    Benutzern bleiben durch die Sysop-Stufe zum Lesen aller Nachrichten geschützt.
user_sec_cmd_s=S) Umfragen
user_sec_cmd_s-status=Stufe zum Beantworten von Umfragen
user_sec_cmd_s-help=
    # S) Umfragen

    Erlaubt das Beantworten von Umfragen. Jede Umfrage kann zusätzlich
    eine eigene Sicherheitsstufe verlangen.
user_sec_cmd_t=T) Übertragungsprotokoll
user_sec_cmd_t-status=Stufe zum Ändern des Übertragungsprotokolls
user_sec_cmd_t-help=
    # T) Übertragungsprotokoll

    Wählt das Standard-Übertragungsprotokoll aus der Protokollliste.
user_sec_cmd_u=U) Datei hochladen
user_sec_cmd_u-status=Stufe für Uploads
user_sec_cmd_u-help=
    # U) Datei hochladen

    Erlaubt Uploads. Für mehrere Dateien gleichzeitig ist zusätzlich
    die Stapelübertragungsstufe nötig. Dateien landen im Uploadverzeichnis der Konferenz.
user_sec_cmd_v=V) Einstellungen ansehen
user_sec_cmd_v-status=Stufe zum Anzeigen eigener Einstellungen
user_sec_cmd_v-help=
    # V) Einstellungen ansehen

    Zeigt den eigenen Datensatz: Sicherheitsstufe, verbrauchte Zeit,
    Übertragungszähler und gewählte Einstellungen.
user_sec_cmd_w=W) Benutzerdaten ändern
user_sec_cmd_w-status=Stufe zum Ändern eigener Benutzerdaten
user_sec_cmd_w-help=
    # W) Benutzerdaten ändern

    Ändert den eigenen Datensatz: Passwort, Ort, Telefonnummern und
    die bei der Registrierung abgefragten Einstellungen.
user_sec_cmd_x=X) Expertenmodus umschalten
user_sec_cmd_x-status=Stufe zum Umschalten des Expertenmodus
user_sec_cmd_x-help=
    # X) Expertenmodus umschalten

    Wechselt zwischen kurzen Experten-Eingabeaufforderungen und vollständigen Menüs.
user_sec_cmd_y=Y) Persönliche Nachrichten
user_sec_cmd_y-status=Stufe zur Suche nach eigenen Nachrichten
user_sec_cmd_y-help=
    # Y) Persönliche Nachrichten

    Sucht nach Nachrichten an den Benutzer. Üblicherweise gilt dieselbe
    Stufe wie für das Lesen von Nachrichten.
user_sec_cmd_z=Z) Dateibeschreibungen suchen
user_sec_cmd_z-status=Stufe zur Suche in Dateibeschreibungen
user_sec_cmd_z-help=
    # Z) Dateibeschreibungen suchen

    Durchsucht den Text der Dateibeschreibungen. Damit findet man Dateien
    anhand ihrer Funktion statt anhand ihres Namens.
user_sec_cmd_chat=Gruppenchat (CHAT)
user_sec_cmd_chat-status=Stufe zur Teilnahme am Gruppenchat
user_sec_cmd_chat-help=
    # Gruppenchat (CHAT)

    Erlaubt die Teilnahme am Chat zwischen Nodes sowie die Befehle,
    mit denen Benutzer ihre Verfügbarkeit für den Chat festlegen.
user_sec_cmd_open_door=Door öffnen (OPEN)
user_sec_cmd_open_door-status=Stufe zum Ausführen von Doors
user_sec_cmd_open_door-help=
    # Door öffnen (OPEN)

    Führt ein Door-Programm aus. Einzelne Doors können eine höhere
    Stufe oder ein eigenes Passwort verlangen.
user_sec_cmd_test_file=Datei prüfen (TEST)
user_sec_cmd_test_file-status=Stufe zum Prüfen von Dateien
user_sec_cmd_test_file-help=
    # Datei prüfen (TEST)

    Prüft, ob ein Archiv auf dem Board unbeschädigt ist,
    bevor Downloadzeit dafür verbraucht wird.
user_sec_cmd_show_user_list=Benutzer suchen/anzeigen (USER)
user_sec_cmd_show_user_list-status=Stufe zur Suche in der Benutzerliste
user_sec_cmd_show_user_list-help=
    # Benutzer suchen/anzeigen (USER)

    Listet oder sucht die in der aktuellen Konferenz registrierten Benutzer.
    Eine Konferenz kann dies mit einer eigenen Einstellung vollständig verbieten.
user_sec_cmd_who=Benutzer anderer Nodes (WHO)
user_sec_cmd_who-status=Stufe zum Anzeigen anderer Nodes
user_sec_cmd_who-help=
    # Benutzer anderer Nodes (WHO)

    Zeigt, wer auf anderen Nodes verbunden ist.
    Bei einem Board mit nur einem Node gibt es nichts anzuzeigen.
user_sec_batch_file_transfer=Stapelübertragungen
user_sec_batch_file_transfer-status=Stufe für Stapelübertragungen
user_sec_batch_file_transfer-help=
    # Stapelübertragungen

    Erlaubt die Übertragung mehrerer Dateien in einem Durchgang, sowohl
    Uploads als auch Downloads. Die jeweilige normale Upload- oder Downloadstufe ist weiterhin nötig.
user_sec_edit_own_messages=Eigene Nachrichten bearbeiten
user_sec_edit_own_messages-status=Stufe zum Bearbeiten eigener Nachrichten
user_sec_edit_own_messages-help=
    # Eigene Nachrichten bearbeiten

    Erlaubt das Bearbeiten eigener gespeicherter Nachrichten, etwa zur
    Korrektur von Tippfehlern oder nach einer abgebrochenen Verbindung.
configuration_options_title=Konfigurationsoptionen
configuration_options_messages=Nachrichten
configuration_options_file_transfer=Dateiübertragungen
configuration_options_system_control=Systemsteuerung
configuration_options_config_switches=Konfigurationsschalter
configuration_options_limits=Grenzwerte
configuration_options_colors=Farben
configuration_options_func_keys=Funktionstasten
configuration_options_ppl_http=PPL HTTP

ppl_http_title=PPL-HTTP-Richtlinien
ppl_http_policy=Zielrichtlinie
ppl_http_policy-status=Erlaubte Ziele für ausgehende Verbindungen von PPL-Programmen wählen.
ppl_http_policy-help=
    # Zielrichtlinie

    Standard ist Öffentlich: PPEs dürfen HTTP- und HTTPS-Hosts verwenden,
    deren aufgelöste Adressen alle öffentlich routbar sind. Die Freigabeliste
    beschränkt den Zugriff auf die unten exakt angegebenen Ursprünge.
    Deaktiviert lehnt alle PPL-HTTP-Anfragen ab. Jede Weiterleitung wird geprüft.
ppl_http_policy_disabled=Deaktiviert
ppl_http_policy_allowlist=Exakte Ursprungsfreigabeliste
ppl_http_policy_public=Öffentliche Ziele (Standard)
ppl_http_allowed_origins=Erlaubte Ursprünge
ppl_http_allowed_origins-status=Kommagetrennte Ursprünge für den Freigabelistenmodus.
ppl_http_allowed_origins-help=
    # Erlaubte Ursprünge

    Exakte Ursprünge, durch Kommas getrennt, zum Beispiel
    https://api.example.com, https://files.example.com:8443

    Schema, Host und Port müssen übereinstimmen. Pfade gehören nicht hierher.
    Ein freigegebener Ursprung darf bewusst auf einen privaten Dienst auflösen.
ppl_http_allow_http=Unverschlüsseltes HTTP erlauben
ppl_http_allow_http-status=Unverschlüsselte http://-Ziele erlauben (standardmäßig aktiviert).
ppl_http_allow_http-help=
    # Unverschlüsseltes HTTP erlauben

    Ausgeschaltet ist nur HTTPS erlaubt. Nur für Ursprünge einschalten,
    die kein TLS verwenden können: HTTP-Anfrageköpfe und -inhalte können
    unterwegs mitgelesen oder verändert werden.
ppl_http_max_response_bytes=Maximale Antwortbytes
ppl_http_max_response_bytes-status=Größter zulässiger Antwortinhalt einer PPL-Anfrage.
ppl_http_max_response_bytes-help=
    # Maximale Antwortbytes

    Die Übertragung stoppt, sobald der dekodierte Antwortinhalt diese
    Grenze überschreitet. Bei Downloads bleibt dann die bisherige Zieldatei erhalten.
ppl_http_max_request_bytes=Maximale Anfragebytes
ppl_http_max_request_bytes-status=Größter zulässiger POST-Inhalt einer PPL-Anfrage.
ppl_http_max_request_bytes-help=
    # Maximale Anfragebytes

    Größere Anfragen werden schon vor dem Verbindungsaufbau abgelehnt.
ppl_http_max_headers=Maximale Kopfzeilen
ppl_http_max_headers-status=Maximale Anzahl der Anfrage- oder Antwortkopfzeilen.
ppl_http_max_headers-help=
    # Maximale Kopfzeilen

    Gilt sowohl für von PPL-Programmen gesendete als auch für vom Server
    zurückgegebene Kopfzeilen.
ppl_http_max_header_bytes=Maximale Kopfzeilenbytes
ppl_http_max_header_bytes-status=Gesamtgröße der Anfrage- oder Antwortkopfzeilen in Bytes.
ppl_http_max_header_bytes-help=
    # Maximale Kopfzeilenbytes

    Gilt unabhängig für jeden Anfrage- und Antwortkopfblock.
ppl_http_connect_timeout=Verbindungsfrist in Sekunden
ppl_http_connect_timeout-status=Maximale Zeit zum Aufbau einer Verbindung.
ppl_http_connect_timeout-help=
    # Verbindungsfrist

    Begrenzt einen einzelnen Verbindungsversuch. Die Gesamtfrist umfasst
    weiterhin DNS, Wartezeiten, Weiterleitungen und Antwortinhalt.
ppl_http_request_timeout=Gesamtfrist in Sekunden
ppl_http_request_timeout-status=Gesamtzeitlimit für einen PPL-HTTP-Vorgang.
ppl_http_request_timeout-help=
    # Gesamtfrist

    Umfasst das Warten auf freie Kapazität, DNS, alle Weiterleitungen,
    den Verbindungsaufbau und den vollständigen Antwortinhalt.
ppl_http_max_redirects=Maximale Weiterleitungen
ppl_http_max_redirects-status=Maximale Anzahl verfolgter Weiterleitungen pro Anfrage.
ppl_http_max_redirects-help=
    # Maximale Weiterleitungen

    Null lehnt Weiterleitungen ab. Jedes erlaubte Weiterleitungsziel
    muss die Zielrichtlinie erfüllen.
ppl_http_max_concurrent=Gleichzeitige Anfragen im Board
ppl_http_max_concurrent-status=Maximale Anzahl gleichzeitiger PPL-HTTP-Vorgänge im gesamten Board.
ppl_http_max_concurrent-help=
    # Gleichzeitige Anfragen im Board

    Zusätzliche Anfragen warten innerhalb ihrer Gesamtfrist.
ppl_http_max_concurrent_node=Gleichzeitige Anfragen pro Node
ppl_http_max_concurrent_node-status=Maximale Anzahl gleichzeitiger PPL-HTTP-Vorgänge eines Nodes.
ppl_http_max_concurrent_node-help=
    # Gleichzeitige Anfragen pro Node

    Verhindert, dass ein einzelner Benutzer oder ein PPE die gesamte
    boardweite Kapazität belegt.
system_control_title=Systemsteuerung
disable_ns_logon=NS-Anmeldung deaktivieren
disable_ns_logon-status=Überspringen der WELCOME-Datei steuern.
disable_ns_logon-help=
    # NS-Anmeldung deaktivieren

    Normalerweise kann ein Benutzer die Grafikfrage mit ;Q beantworten,
    um die Begrüßung zu überspringen, und mit ;NS auch die Neuigkeiten auslassen.
    Einschalten, um die Nonstop-Abkürzung zu sperren, damit alle die
    Neuigkeiten mindestens einmal sehen.
is_multi_lingual=Mehrsprachiger Betrieb
is_multi_lingual-status=Mehrsprachigen Betrieb aktivieren
is_multi_lingual-help=
    # Mehrsprachiger Betrieb

    Bietet bei der Anmeldung eine Sprachauswahl an. Eingabeaufforderungen
    und Anzeigedateien stammen dann aus der gewählten Sprache.
allow_alias_change=Aliasänderung erlauben
allow_alias_change-status=Ein fester Alias vermeidet Verwechslungen bei Nachrichten.
allow_alias_change-help=
    # Aliasänderung erlauben

    Legt fest, ob ein einmal gewählter Alias geändert werden darf.
    Ausgeschaltet kann niemand alle paar Tage unter neuem Namen auftreten;
    erst diese Beständigkeit macht den Alias für andere Benutzer nützlich.
is_closed_board=Geschlossenes Board
is_closed_board-status=Keine neuen Benutzer zulassen.
is_closed_board-help=
    # Geschlossenes Board

    Verhindert die Aufnahme neuer Benutzer. Für neue Namen wird kein
    Datensatz mehr angelegt; nur vorhandene Benutzer können sich anmelden.
enforce_daily_time_limit=Tägliches Zeitlimit durchsetzen
enforce_daily_time_limit-status=Zwischen Sitzungs- und Tageszeitlimit umschalten.
enforce_daily_time_limit-help=
    # Tägliches Zeitlimit durchsetzen

    Bereits heute verbrauchte Minuten werden auf das Zeitkontingent
    angerechnet. Ein erneuter Anruf gewährt also nicht wieder die volle
    Sitzungszeit. Ausgeschaltet beginnt jeder Anruf mit voller Sitzungszeit.
enforce_transfer_limits=Übertragungslimits durchsetzen
enforce_transfer_limits-status=PWRD-Byte-, Datei- und Verhältnislimits auf Downloads anwenden
enforce_transfer_limits-help=Standardmäßig ausgeschaltet, damit importierte Boards erst nach Prüfung der Limits Downloads verweigern.
allow_password_failure_comment=Kommentar bei Passwortfehler
allow_password_failure_comment-status=Nach erfolglosen Passwortversuchen an den Sysop schreiben.
allow_password_failure_comment-help=
    # Kommentar bei Passwortfehler

    Nach ausgeschöpften Passwortversuchen darf der Benutzer einen Kommentar
    an den Sysop schreiben, statt nur getrennt zu werden. So kann er bei
    einem vergessenen Passwort um Hilfe bitten.
password_storage_method=Passwortspeicherung
password_storage_method-status=Klartextspeicherung ein-/ausschalten
password_storage_method-help=
    # Passwortspeicherung

    Legt fest, wie Passwörter in der Benutzerdatei gespeichert werden.
    PCBoard verwendete Klartext, lesbar für jeden mit Dateizugriff.
    bcrypt und Argon2 speichern stattdessen einen Hash, sodass eine
    gestohlene Benutzerdatei nicht unmittelbar die Passwörter preisgibt.
    Vorhandene Passwörter werden bei der nächsten Anmeldung des jeweiligen Benutzers umgewandelt.
password_storage_method_plain_text=Klartext
password_storage_method_bcrypt=BCrypt-Hash
password_storage_method_argon2=Argon2-Hash
guard_logoff=Abmeldung bestätigen
guard_logoff-status=Bei 'g' vor der Abmeldung nachfragen.
guard_logoff-help=
    # Abmeldung bestätigen

    Verlangt vor dem Trennen eine Bestätigung des Befehls G und schützt
    so vor versehentlicher Abmeldung. BYE meldet ohne Rückfrage ab.
confirm_caller_name=Benutzernamen bestätigen
confirm_caller_name-status=Gefundenen Datensatz zeigen und Tippfehler korrigieren lassen
confirm_caller_name-help=Fängt falsch geschriebene Namen ab, die sonst ein zweites Konto erzeugen würden.
reread_sec_level_on_join=Stufenlimits bei Beitritt neu lesen
reread_sec_level_on_join-status=Limits erneut anwenden, wenn eine Konferenz die Stufe ändert
reread_sec_level_on_join-help=Konferenzen können die Sicherheitsstufe des Benutzers erhöhen oder senken.
max_msg_lines=Maximale Nachrichtenzeilen
max_msg_lines-status=Maximale Zeilenzahl, die Benutzer im Nachrichteneditor bearbeiten können.
max_msg_lines-help=
    # Maximale Nachrichtenzeilen

    Erlaubte Zeilenanzahl einer Nachricht, zwischen 17 und 400.
    Die Grenze gilt auch für Nachrichten aus hochgeladenen Nachrichtenpaketen.
disable_message_scan_prompt=Nachrichtensuche ohne Rückfrage
disable_message_scan_prompt-status=Nachrichtensuche bei Anmeldung und Konferenzwechsel steuern.
disable_message_scan_prompt-help=
    # Nachrichtensuche ohne Rückfrage

    Unterdrückt die Frage nach der Nachrichtensuche bei der Anmeldung
    und beim Beitritt zu einer noch nicht durchsuchten Konferenz.
allow_esc_codes=ESC-Codes in Nachrichten erlauben
allow_esc_codes-status=Escape-Sequenzen in Nachrichten erlauben.
allow_esc_codes-help=
    # ESC-Codes in Nachrichten erlauben

    Erlaubt rohe Escape-Sequenzen im Nachrichtentext. @X-Farbcodes sind
    kürzer und ohnehin erlaubt. Im reinen Textmodus erscheinen Escape-Codes
    als Zeichensalat, deshalb bleibt diese Option bei den meisten Boards aus.
allow_carbon_copy=Nachrichtenkopien erlauben
allow_carbon_copy-status=Mit SC Nachrichten an mehrere Benutzer senden.
allow_carbon_copy-help=
    # Nachrichtenkopien erlauben

    Erlaubt mit SC das Speichern einer Nachricht an mehrere Empfänger.
    Benutzern sollte empfohlen werden, dies nur für private Nachrichten
    zu verwenden: Öffentliche Nachrichten können ohnehin alle lesen.
validate_to_name=Empfängernamen prüfen
validate_to_name-status=Den Namen im TO:-Feld von Nachrichten prüfen.
validate_to_name-help=
    # Empfängernamen prüfen

    Prüft die Eingabe bei TO: anhand der Benutzerdatei und erlaubt die
    Korrektur von Tippfehlern, statt eine unerreichbare Nachricht zu schreiben.
    In Echomail-Konferenzen erfolgt diese Prüfung nicht.
default_quick_personal_scan=Persönliche Suche standardmäßig (Q)
default_quick_personal_scan-status=Bei persönlicher Nachrichtensuche (Q) als Vorgabe verwenden
default_quick_personal_scan-help=
    # Persönliche Suche standardmäßig (Q)

    Bei der Suche nach eigenen Nachrichten wird standardmäßig nur die
    kurze Kopfliste angezeigt, statt die Nachrichten sofort zu lesen.
default_scan_all_selected_confs_at_login=Bei Anmeldung alle Konferenzen suchen
default_scan_all_selected_confs_at_login-status=Bei der ersten Anmeldung aktuelle oder alle Konferenzen durchsuchen.
default_scan_all_selected_confs_at_login-help=
    # Bei Anmeldung alle Konferenzen durchsuchen

    Die Suche bei der Anmeldung umfasst alle vom Benutzer ausgewählten
    Konferenzen statt nur der aktuellen. Passt gut zur schnellen Suche als Vorgabe.
prompt_to_read_mail=Neue persönliche Nachrichten anbieten
prompt_to_read_mail-status=Lesen neuer Nachrichten an den Benutzer anbieten.
prompt_to_read_mail-help=
    # Neue persönliche Nachrichten anbieten

    Nach der Anzeige, welche Konferenzen neue persönliche Nachrichten
    enthalten, wird deren sofortiges Lesen angeboten. Der Benutzer
    muss sie so nicht selbst aufsuchen.
force_comments_to_main=Kommentare in Hauptkonferenz
force_comments_to_main-status=Kommentare an den Sysop immer in der Hauptkonferenz schreiben
force_comments_to_main-help=Verhindert, dass Kommentare über verschiedene Konferenzen verstreut werden.
update_last_read_pointer=Lesezeiger beim Lesen weitersetzen
update_last_read_pointer-status=Beim Lesen einer Nachricht den Lesezeiger weitersetzen
update_last_read_pointer-help=Bestimmt, was die nächste Suche nach neuen Nachrichten anzeigt.
keyboard_timeout=Eingabe-Timeout (Minuten)
keyboard_timeout-status=0=deaktiviert
keyboard_timeout-help=
    # Eingabe-Timeout

    Nach so vielen Minuten ohne Eingabe wird die Verbindung beendet.
    Das gibt den Node frei, wenn jemand weggeht. Drei bis fünf Minuten
    passen für die meisten Boards. 0 schaltet das Limit aus; der Benutzer
    kann dann bis zum Ablauf seiner Zeit untätig bleiben.
max_number_upload_descr_lines=Maximale Beschreibungszeilen
max_number_upload_descr_lines-status=0=deaktiviert
max_number_upload_descr_lines-help=
    # Maximale Beschreibungszeilen

    So viele Zeilen darf ein Benutzer zur Beschreibung eines Uploads schreiben.
password_expire_days=Passwortwechsel nach Tagen
password_expire_days-status=0=deaktiviert
password_expire_days-help=
    # Passwortwechsel nach Tagen

    Nach dieser Anzahl von Tagen muss der Benutzer vor dem Fortfahren
    ein neues Passwort wählen. Bei 0 bleibt ein Passwort unbegrenzt gültig.
password_expire_warn_days=Warntage vor Passwortwechsel
password_expire_warn_days-status=0=deaktiviert
password_expire_warn_days-help=
    # Warntage vor Passwortwechsel

    So viele Tage vor dem erzwungenen Wechsel wird gewarnt, damit der
    Benutzer nicht sofort ein neues Passwort erfinden muss. 0 schaltet die Warnung aus.
min_pwd_length=Mindestlänge des Passworts
min_pwd_length-status=Kürzestes erlaubtes Passwort
min_pwd_length-help=
    # Mindestlänge des Passworts

    Kürzestes Passwort, das bei der Registrierung oder einer Passwortänderung
    akzeptiert wird. Bereits gespeicherte Passwörter werden nicht dagegen geprüft.
disallow_batch_uploads=Stapel-Uploads verbieten
disallow_batch_uploads-status=Stapel-Uploads möglichst nicht verbieten.
disallow_batch_uploads-help=
    # Stapel-Uploads verbieten

    Verweigert mehrere Uploads pro Durchgang. Dies hilft vor allem bei
    Clients, deren Dateinamen die Übertragung nicht korrekt überstehen.
    Es erschwert allerdings Uploads; die meisten Boards lassen Stapel-Uploads deshalb zu.
promote_to_batch_transfers=Automatisch Stapelübertragung
promote_to_batch_transfers-status=Bei gewähltem Stapelprotokoll automatisch Stapelübertragung verwenden.
promote_to_batch_transfers-help=
    # Automatisch Stapelübertragung

    Wandelt U oder D in die Stapelvariante um, wenn ein Stapelprotokoll
    gewählt ist. So lassen sich mehrere Dateien pro Befehl angeben.
upload_credit_time=Zeitgutschrift für Uploads
upload_credit_time-status=Standard 1.0 hält die Uhr während des Uploads an.
upload_credit_time-help=
    # Zeitgutschrift für Uploads

    Anteil der Uploadzeit, der zurückgegeben wird. Bei 1.0 bleibt so viel
    Zeit wie vor dem Upload. 2.5 schreibt für jede Uploadminute
    zweieinhalb Minuten gut.
upload_credit_bytes=Bytegutschrift für Uploads
upload_credit_bytes-status=Standard 1.0 schreibt pro Uploadbyte ein Downloadbyte gut.
upload_credit_bytes-help=
    # Bytegutschrift für Uploads

    Downloadkontingent pro hochgeladenem Byte. 1.0 gibt ein Byte pro
    Uploadbyte zurück; höhere Werte belohnen Uploads stärker. Die Gutschrift
    wird auf das tägliche Downloadlimit angerechnet und verfällt am Tagesende.
display_uploader=Uploader in Beschreibung nennen
display_uploader-status=Uploader in der Dateibeschreibung nennen.
display_uploader-help=
    # Uploader in Beschreibung nennen

    Fügt jeder Uploadbeschreibung eine Zeile mit dem Benutzernamen hinzu.
    So lässt sich später leicht erkennen, wer eine Datei beigesteuert hat.
strip_colors_in_descriptions=Farben aus Beschreibungen entfernen
strip_colors_in_descriptions-status=Farbcodes aus FILE_ID.DIZ entfernen.
strip_colors_in_descriptions-help=FILE_ID.DIZ kann beliebige Farben des Autors enthalten. Ein darin enthaltener Farbreset stellt die Terminalvorgabe statt der Boardfarbe für Dateilisten wieder her. Einschalten, damit Listen in den Boardfarben bleiben. Abstände und Zeichengrafiken bleiben in beiden Fällen erhalten.
verify_files_uploaded=Uploads überprüfen
verify_files_uploaded-status=Dateien nach dem Upload prüfen.
verify_files_uploaded-help=
    # Uploads überprüfen

    Prüft einen Upload vor der Freigabe. An dieser Stelle führte
    PCBoard einen Virenscanner oder Archivtest aus.
disable_drive_size_check=Freiplatzprüfung deaktivieren
disable_drive_size_check-status=Deaktiviert auch die Freiplatzmeldung.
disable_drive_size_check-help=
    # Freiplatzprüfung deaktivieren

    Unterdrückt vor Uploads die Prüfung und Anzeige des freien Speicherplatzes.
    Nur sinnvoll, wenn der freie Platz nicht verlässlich gemessen werden kann.
stop_uploads_free_space=Uploadstopp bei freiem Platz unter
stop_uploads_free_space-status=Mindestens benötigter freier Speicherplatz in KB für Uploads.
stop_uploads_free_space-help=
    # Uploadstopp bei freiem Platz unter

    Uploads werden verweigert, wenn das Uploadlaufwerk weniger als diese
    Anzahl Kilobytes frei hat. So legt eine volle Platte nicht das Board lahm.
    0 schaltet das Limit aus.
disable_registration_edits=Anmeldefilter deaktivieren
disable_registration_edits-status=Filterung der Anmeldeeingaben deaktivieren
disable_registration_edits-help=
    # Anmeldefilter deaktivieren

    Schaltet die Filterung der Anmeldeeingaben aus. Der Filter schützt
    gegen Leitungsstörungen. Nur ausschalten, wenn Benutzer Zeichen
    benötigen, die der Filter sonst entfernt.
disable_high_ascii_filter=High-ASCII-Filter deaktivieren
disable_high_ascii_filter-status=Filterung von High-ASCII-Zeichen deaktivieren.
disable_high_ascii_filter-help=
    # High-ASCII-Filter deaktivieren

    Lässt Zeichen oberhalb des einfachen ASCII-Bereichs durch, wie sie
    für Sprachen mit Akzenten benötigt werden. Sonst entfernt der Filter
    sie als vermeintliche Leitungsstörungen.
default_graphics_at_login=Grafik bei Anmeldung als Vorgabe
default_graphics_at_login-status=Standardantwort auf die Grafikfrage festlegen
default_graphics_at_login-help=
    # Grafik bei Anmeldung als Vorgabe

    Macht Ja zur Standardantwort auf die Grafikfrage. Wer nur Return
    drückt, erhält damit eine farbige Darstellung.
non_graphics=Nur Textmodus verwenden
non_graphics-status=Alle Grafik deaktivieren
non_graphics-help=
    # Nur Textmodus verwenden

    Betreibt das Board für alle im reinen Textmodus, ohne Grafikfrage.
    Farben und Blockzeichen werden nie gesendet. Das eignet sich für
    Terminals, die solche Zeichen nicht darstellen können.
exclude_local_calls_stats=Lokale Anmeldungen nicht zählen
exclude_local_calls_stats-status=Lokale Anmeldungen von der Statistik ausschließen
exclude_local_calls_stats-help=
    # Lokale Anmeldungen nicht zählen

    Nimmt lokale Anmeldungen samt zugehörigen Übertragungen und Nachrichten
    von der Statistik aus, damit eigene Tests des Sysops die Zahlen nicht beschönigen.
display_news_behavior=NEWS-Anzeige
display_news_behavior-status=Y (bei Änderung), N (einmal täglich), A (immer), X (nie)
display_news_behavior-help=
    # NEWS-Anzeige

    Wann Neuigkeiten bei der Anmeldung angezeigt werden:

    - Y: wenn die Datei neuer als der letzte Anruf ist
    - N: einmal täglich
    - A: bei jedem Anruf
    - X: nie
display_userinfo_at_login=Benutzerdaten bei Anmeldung
display_userinfo_at_login-status=Statistik (V) bei der Anmeldung anzeigen.
display_userinfo_at_login-help=
    # Benutzerdaten bei Anmeldung

    Zeigt direkt nach der Anmeldung die eigene Statistik wie bei V:
    letzter Anruf, Anzahl der Anrufe und Übertragungszähler.
force_intro_on_join=INTRO bei Beitritt erzwingen
force_intro_on_join-status=INTRO bei jedem Konferenzbeitritt anzeigen.
force_intro_on_join-help=
    # INTRO bei Beitritt erzwingen

    Normalerweise kann ein Benutzer die Konferenzeinleitung durch ein
    angehängtes Q am Beitrittsbefehl überspringen. Hiermit wird die Anzeige
    erzwungen, etwa wenn die Einleitung Konferenzregeln enthält.
scan_new_blt=Neue Bulletins suchen
scan_new_blt-status=Bei der Anmeldung nach neuen Bulletins suchen.
scan_new_blt-help=
    # Neue Bulletins suchen

    Sucht bei der Anmeldung nach noch nicht gesehenen Bulletins und weist
    darauf hin. Ausschalten beschleunigt die Anmeldung bei vielen Bulletins,
    allerdings können Benutzer dann neue Bulletins übersehen.
capture_grp_chat_session=Gruppenchat protokollieren
capture_grp_chat_session-status=GROUP CHAT in einer Datei protokollieren
capture_grp_chat_session-help=
    # Gruppenchat protokollieren

    Schreibt die Eingaben im Gruppenchat in eine Datei,
    damit der Sysop die Sitzung später nachlesen kann.
allow_handle_in_grpchat=Pseudonyme im Gruppenchat
allow_handle_in_grpchat-status=Pseudonyme in GROUP CHAT erlauben
allow_handle_in_grpchat-help=
    # Pseudonyme im Gruppenchat

    Benutzer dürfen für den Gruppenchat einen Anzeigenamen wählen,
    statt unter ihrem Vornamen aufzutreten.
call_log=Anrufprotokoll schreiben
call_log-status=Jede Anmeldung im Anrufprotokoll erfassen
call_log-help=Die Datei wird unter Dateipfade – Systemdateien festgelegt.
page_notification_command=Befehl bei Sysop-Ruf
page_notification_command-status=Shell-Befehl, der bei einem Sysop-Ruf einmal ausgeführt wird
page_notification_command-help=
    # Befehl bei Sysop-Ruf

    Optionaler vertrauenswürdiger Shell-Befehl, der zu Beginn eines
    Sysop-Rufs einmal ausgeführt wird. Leer lassen, um externe Meldungen
    zu deaktivieren. ICB_PAGE_NODE enthält die Node-Nummer,
    ICB_PAGE_USER den Benutzernamen.

    Beispiel für einen Linux-Desktop:
    notify-send "Icy Board" "Sysop-Ruf von $ICB_PAGE_USER auf Node $ICB_PAGE_NODE"
log_caller_number=Anrufernummer protokollieren
log_caller_number-status=Anrufernummer der Sitzung im Anrufprotokoll erfassen
log_caller_number-help=
    # Anrufernummer protokollieren

    Schreibt die fortlaufende Anrufnummer ins Protokoll. Damit lässt sich
    ein Protokolleintrag einer bestimmten Sitzung zuordnen.
log_connect_string=Verbindung protokollieren
log_connect_string-status=Verbindungsart im Anrufprotokoll erfassen
log_connect_string-help=
    # Verbindung protokollieren

    Hält fest, wie der Benutzer das Board erreicht hat. So lassen sich
    Beschwerden über langsame oder gestörte Sitzungen auf die Verbindungsart zurückführen.
log_security_level=Sicherheitsstufe protokollieren
log_security_level-status=Sicherheitsstufe im Anrufprotokoll erfassen
log_security_level-help=
    # Sicherheitsstufe protokollieren

    Schreibt die Sicherheitsstufe bei der Anmeldung ins Anrufprotokoll.
    Sie kann sich beim Konferenzbeitritt erneut ändern.

# ICBSetup -> Konfigurationsoptionen -> Farben
default_color=Standardfarbe
default_color-status=Standardfarbe für Eingabeaufforderungen.
default_color-help=
    # Standardfarbe

    Zu dieser Farbe kehrt das Board nach Eingabeaufforderungen oder
    Anzeigedateien mit eigenen Farben zurück. Alles ohne eigene
    Farbfestlegung wird in dieser Farbe ausgegeben.
msg_hdr_date=Nachrichtenkopf DATE
msg_hdr_date-status=Farbe der Datumszeile im Nachrichtenkopf
msg_hdr_date-help=
    # Nachrichtenkopf DATE

    Farbe der Datumszeile im Kopf oberhalb einer Nachricht.
msg_hdr_to=Nachrichtenkopf TO
msg_hdr_to-status=Farbe der Empfängerzeile im Nachrichtenkopf
msg_hdr_to-help=
    # Nachrichtenkopf TO

    Farbe der Empfängerzeile im Kopf oberhalb einer Nachricht.
msg_hdr_from=Nachrichtenkopf FROM
msg_hdr_from-status=Farbe der Absenderzeile im Nachrichtenkopf
msg_hdr_from-help=
    # Nachrichtenkopf FROM

    Farbe der Absenderzeile im Kopf oberhalb einer Nachricht.
msg_hdr_subj=Nachrichtenkopf SUBJ
msg_hdr_subj-status=Farbe der Betreffzeile im Nachrichtenkopf
msg_hdr_subj-help=
    # Nachrichtenkopf SUBJ

    Farbe der Betreffzeile im Kopf oberhalb einer Nachricht.
msg_hdr_read=Nachrichtenkopf READ
msg_hdr_read-status=Farbe der Lesestatuszeile im Nachrichtenkopf
msg_hdr_read-help=
    # Nachrichtenkopf READ

    Farbe der Zeile, die angibt, ob eine Nachricht gelesen wurde.
msg_hdr_conf=Nachrichtenkopf CONF
msg_hdr_conf-status=Farbe der Konferenzzeile im Nachrichtenkopf
msg_hdr_conf-help=
    # Nachrichtenkopf CONF

    Farbe der Konferenzzeile im Kopf oberhalb einer Nachricht.
file_head=Dateilistenkopf
file_head-status=Farbe des Dateilistenkopfs
file_head-help=
    # Dateilistenkopf

    Farbe der Überschrift oberhalb einer Dateiliste.
file_name=Dateiname
file_name-status=Farbe des Dateinamens
file_name-help=
    # Dateiname

    Farbe der Dateinamensspalte in einer Dateiliste.
file_size=Dateigröße
file_size-status=Farbe der Dateigröße
file_size-help=
    # Dateigröße

    Farbe der Größenspalte in einer Dateiliste.
file_date=Dateidatum
file_date-status=Farbe des Dateidatums
file_date-help=
    # Dateidatum

    Farbe der Datumsspalte in einer Dateiliste.
file_description=Erste Beschreibungszeile
file_description-status=Farbe der ersten Dateibeschreibungszeile
file_description-help=
    # Erste Beschreibungszeile

    Farbe der ersten Beschreibungszeile, die in Dateilisten
    als Kurzbeschreibung verwendet wird.
file_duplicate=Doppelte Datei
file_duplicate-status=Farbe für Hinweise auf doppelte Dateien
file_duplicate-help=
    # Doppelte Datei

    Farbe der Hinweise auf doppelte Dateien in einer Verzeichnisliste.
file_text=Text in Dateilisten
file_text-status=Farbe für Text zwischen Dateieinträgen
file_text-help=
    # Text in Dateilisten

    Farbe des normalen Textes zwischen den Einträgen einer Dateiliste.
file_deleted=Gelöschte Datei
file_deleted-status=Farbe der Löschkennzeichnung in Dateilisten
file_deleted-help=
    # Gelöschte Datei

    Farbe des Wortes, das einen Eintrag ohne noch vorhandene Datei kennzeichnet.

# ICBSetup -> Dateipfade
file_locations_title=Dateipfade
file_locations_files_dirs=Systemdateien und Verzeichnisse
file_locations_config_files=Konfigurationsdateien
file_locations_display_files=Anzeigedateien
file_locations_surveys=Registrierungs-/An-/Abmeldeumfragen
paths_conferences=Konferenzdaten
paths_conferences-status=Name/Pfad der Konferenzdaten
paths_conferences-help=
    # Konferenzdaten

    Diese Datei enthält alle Konferenzen des Boards samt Einstellungen.
    Ohne sie gibt es nur die Hauptkonferenz.
paths_users_file=Benutzerdatei
paths_users_file-status=Name/Pfad der Benutzerdatei
paths_users_file-help=
    # Benutzerdatei

    Enthält alle Benutzerdatensätze: Namen, Passwörter, Sicherheitsstufen,
    Statistiken und Konferenzregistrierungen. Diese Datei darf ein Board
    keinesfalls verlieren; sie gehört unbedingt in die Datensicherung.
paths_group_file=Gruppendatei
paths_group_file-status=Name/Pfad der Gruppendatei
paths_group_file-help=
    # Gruppendatei

    Definiert die Gruppen, denen Benutzer angehören können. Sicherheitsausdrücke
    prüfen Gruppen, wenn der Zugriff nicht nur von einer Stufe abhängen soll.
paths_caller_log=Anrufprotokoll
paths_caller_log-status=Name/Pfad des Anrufprotokolls
paths_caller_log-help=
    # Anrufprotokoll

    Hier protokolliert das Board die Ereignisse jedes Anrufs.
    Erste Anlaufstelle, wenn ein Benutzer ungewöhnliches Verhalten meldet.
paths_transfer_log=Übertragungsprotokoll
paths_transfer_log-status=Name/Pfad des Protokolls aller abgeschlossenen Übertragungen
paths_transfer_log-help=
    # Übertragungsprotokoll

    Erfasst jeden abgeschlossenen Upload und Download samt Benutzer und Datei.
    Zeigt, wer was wann übertragen hat.
paths_statistic_file=Statistikdatei
paths_statistic_file-status=Name/Pfad der Statistikdatei
paths_statistic_file-help=
    # Statistikdatei

    Speichert die laufenden Summen des Anruf-Warteschirms:
    Anrufe, Nachrichten und Übertragungen.
paths_icbtext=ICBTEXT-Datei
paths_icbtext-status=Name/Pfad der ICBTEXT-Datei
paths_icbtext-help=
    # ICBTEXT-Datei

    Enthält alle Eingabeaufforderungen und Meldungen für Benutzer.
    Mit mkicbtxt bearbeiten, um Texte umzuformulieren oder zu übersetzen.
paths_tmp_files=Temporäre Arbeitsdateien
paths_tmp_files-status=Verzeichnis für temporäre Arbeitsdateien
paths_tmp_files-help=
    # Temporäre Arbeitsdateien

    Verzeichnis für nur während einer Sitzung benötigte Dateien,
    etwa ein gerade entstehendes Nachrichtenpaket. Es sollte auf einem
    schnellen Datenträger mit ausreichend freiem Platz liegen.
paths_help_path=Hilfedateien
paths_help_path-status=Verzeichnis der Hilfedateien
paths_help_path-help=
    # Hilfedateien

    Enthält die mit H erreichbaren Hilfedateien.
    Jede Datei heißt wie der Befehl, den sie erklärt.
paths_security_file_path=Sicherheits-Anzeigedateien
paths_security_file_path-status=Verzeichnis der Sicherheitsdateien für die Anmeldung
paths_security_file_path-help=
    # Sicherheits-Anzeigedateien

    Enthält Anzeigen für aus Sicherheitsgründen verweigerte Aktionen.
    So kann das Board eine Ablehnung erklären, statt nur Nein zu sagen.
paths_email_msg_base=Private Nachrichtenbasis
paths_email_msg_base-status=Pfad der privaten Nachrichtenbasis
paths_email_msg_base-help=
    # Private Nachrichtenbasis

    Nachrichtenbasis für private Post zwischen Benutzern.
    Die Befehle für persönliche Nachrichten lesen und schreiben hier.
paths_command_display_path=Befehls-Anzeigedateien
paths_command_display_path-status=Verzeichnis der Befehls-Anzeigedateien
paths_command_display_path-help=
    # Befehls-Anzeigedateien

    Enthält Bildschirme, die vor der Ausführung eines Befehls erscheinen.
    Die jeweilige Datei wird anhand des Befehlsnamens gefunden.
paths_newask_survey=Registrierungsumfrage
paths_newask_survey-status=Name/Pfad der NEWASK-Umfragedatei
paths_newask_survey-help=
    # Registrierungsumfrage

    Zusätzliche Fragen bei der Registrierung neben den Standardfragen.
    Hier fragt das Board benötigte Angaben ab, bevor es Zugriff gewährt.
paths_newask_answer=Registrierungsantworten
paths_newask_answer-status=Name/Pfad der Antworten zur NEWASK-Umfrage
paths_newask_answer-help=
    # Registrierungsantworten

    Hier werden die Antworten zur Registrierung für den Sysop gesammelt.
paths_logon_survey=Anmeldeumfrage
paths_logon_survey-status=Name/Pfad der Anmeldeumfrage
paths_logon_survey-help=
    # Anmeldeumfrage

    Fragen bei jeder Anmeldung. Leer lassen, wenn das Board nicht
    wirklich bei jedem Anruf etwas erfragen muss.
paths_logon_answer=Anmeldeantworten
paths_logon_answer-status=Name/Pfad der Antworten zur Anmeldeumfrage
paths_logon_answer-help=
    # Anmeldeantworten

    Hier werden die Antworten auf die Anmeldefragen gespeichert.
paths_logoff_survey=Abmeldeumfrage
paths_logoff_survey-status=Name/Pfad der Abmeldeumfrage
paths_logoff_survey-help=
    # Abmeldeumfrage

    Fragen beim Abmelden, etwa danach, wie der Besuch gefallen hat.
paths_logoff_answer=Abmeldeantworten
paths_logoff_answer-status=Name/Pfad der Antworten zur Abmeldeumfrage
paths_logoff_answer-help=
    # Abmeldeantworten

    Hier werden die Antworten auf die Abmeldefragen gespeichert.

paths_welcome=WELCOME-Datei
paths_welcome-status=Name/Pfad der WELCOME-Datei
paths_welcome-help=
    # WELCOME-Datei

    Erster Bildschirm nach dem Verbindungsaufbau, noch vor der Eingabe
    des Namens. Er ist die Eingangstür des Boards.
paths_newuser=NEWUSER-Datei
paths_newuser-status=Name/Pfad der NEWUSER-Datei
paths_newuser-help=
    # NEWUSER-Datei

    Wird unbekannten Benutzern vor der Frage nach der Registrierung gezeigt.
    Hier gehören Regeln und die Vorteile einer Registrierung hin.
paths_closed=CLOSED-Datei
paths_closed-status=Name/Pfad der CLOSED-Datei
paths_closed-help=
    # CLOSED-Datei

    Wird neuen Benutzern bei geschlossenem Board angezeigt. Sie erklärt,
    warum keine Registrierung möglich ist und wie ein Konto beantragt werden kann.
paths_expire_warning=WARNING-Datei
paths_expire_warning-status=Name/Pfad der WARNING-Datei
paths_expire_warning-help=
    # WARNING-Datei

    Wird während der Warntage vor Ablauf eines Abonnements angezeigt.
    Hier sollte stehen, wie es verlängert werden kann.
paths_expired=EXPIRED-Datei
paths_expired-status=Name/Pfad der EXPIRED-Datei
paths_expired-help=
    # EXPIRED-Datei

    Wird nach Ablauf eines Abonnements angezeigt, sobald der Benutzer
    auf die Sicherheitsstufe für abgelaufene Konten zurückgestuft wurde.
paths_conf_join_menu=Konferenzauswahlmenü
paths_conf_join_menu-status=Name/Pfad des Konferenzauswahlmenüs
paths_conf_join_menu-help=
    # Konferenzauswahlmenü

    Liste der Konferenzen, die auf Anfrage angezeigt wird.
paths_conf_chat_intro_file=Gruppenchat-Einleitung
paths_conf_chat_intro_file-status=Name/Pfad der Gruppenchat-Einleitung
paths_conf_chat_intro_file-help=
    # Gruppenchat-Einleitung

    Wird beim Betreten des Gruppenchats angezeigt. Hier gehören die
    Chatregeln und ein Hinweis zum Verlassen des Chats hin.
paths_conf_chat_menu=Gruppenchat-Menü
paths_conf_chat_menu-status=Name/Pfad des Gruppenchat-Menüs
paths_conf_chat_menu-help=
    # Gruppenchat-Menü

    Liste der im Gruppenchat verfügbaren Befehle.
paths_conf_chat_actions_menu=Chat-Aktionsmenü
paths_conf_chat_actions_menu-status=Name/Pfad des Chat-Aktionsmenüs
paths_conf_chat_actions_menu-help=
    # Chat-Aktionsmenü

    Liste der Aktionen im Gruppenchat, etwa Winken oder Grinsen.
paths_no_ansi=NOANSI-Warnung
paths_no_ansi-status=Name/Pfad der NOANSI-Warndatei
paths_no_ansi-help=
    # NOANSI-Warnung

    Wird angezeigt, wenn ein Benutzer ohne ANSI-fähiges Terminal
    eine Funktion aufruft, die ANSI benötigt.
paths_pwrd_sec_level_file=PWRD-/Sicherheitsdatei
paths_pwrd_sec_level_file-status=Name/Pfad der PWRD-/Sicherheitsdatei
paths_pwrd_sec_level_file-help=
    # PWRD-/Sicherheitsdatei

    Definiert Zeit pro Tag, Downloadkontingent, Verhältnisse und Passwort
    jeder Sicherheitsstufe. Diese Grenzen werden hier festgelegt,
    nicht in jedem einzelnen Benutzerdatensatz.
paths_trashcan_user=Namenssperrliste
paths_trashcan_user-status=Name/Pfad der Namenssperrliste
paths_trashcan_user-help=
    # Namenssperrliste

    Nicht registrierbare Namen, einer pro Zeile. Verhindert beleidigende
    Namen oder solche, mit denen sich Benutzer als Sysop ausgeben könnten.
paths_trashcan_upload_files=Upload-Sperrliste
paths_trashcan_upload_files-status=Name/Pfad der Upload-Sperrliste
paths_trashcan_upload_files-help=
    # Upload-Sperrliste

    Dateinamen, die nicht hochgeladen werden dürfen; Platzhalter sind erlaubt.
    Bei einem Treffer erhält der Benutzer den Hinweis, dass die Datei unerwünscht ist.
paths_trashcan_passwords=Passwortsperrliste
paths_trashcan_passwords-status=Name/Pfad der Passwortsperrliste
paths_trashcan_passwords-help=
    # Passwortsperrliste

    Verbotene Passwörter, eines pro Zeile. Hier gehören leicht erratbare
    Passwörter hin, damit sie keinen Zugriff auf ein Konto ermöglichen.
paths_trashcan_email=E-Mail-Sperrliste
paths_trashcan_email-status=Name/Pfad der E-Mail-Sperrliste
paths_trashcan_email-help=
    # E-Mail-Sperrliste

    E-Mail-Adressen, die das Board nicht von Benutzern akzeptiert.
paths_vip_users=VIP-Benutzerdatei
paths_vip_users-status=Name/Pfad der VIP-Benutzerdatei
paths_vip_users-help=
    # VIP-Benutzerdatei

    Namen, über deren Anmeldung der Sysop informiert werden möchte.
    Meldet sich einer davon an, weist das Board darauf hin,
    damit wichtige Benutzer nicht übersehen werden.
paths_protocol_data_file=Protokolldatei
paths_protocol_data_file-status=Name/Pfad der Protokolldatei
paths_protocol_data_file-help=
    # Protokolldatei

    Liste der auswählbaren Übertragungsprotokolle.
    Auf dieser Zeile F2 zum Bearbeiten drücken.
paths_language_file=Sprachdatei
paths_language_file-status=Name/Pfad der Mehrsprachigkeitsdatei
paths_language_file-help=
    # Sprachdatei

    Tabelle der angebotenen Sprachen mit der jeweiligen Erweiterung
    ihrer Anzeigedateien und Eingabeaufforderungen.
paths_command_file=CMD.LST-Datei
paths_command_file-status=Name/Pfad der CMD.LST-Datei
paths_command_file-help=
    # CMD.LST-Datei

    Eigene Befehle zusätzlich zu den eingebauten. Damit erhält etwa
    ein PPE oder Door einen Namen, den Benutzer an der Eingabeaufforderung eingeben können.
connection_info_title=Verbindungen
connection_info_telnet=Telnet
connection_info_ssh=SSH
connection_info_websockets=WebSockets
connection_info_secure_websockets=Sichere WebSockets
connection_info_enabled=Aktiviert
connection_info_enabled-status=Board über diese Verbindungsart erreichbar machen
connection_info_enabled-help=
    # Aktiviert

    Legt fest, ob das Board Verbindungen dieser Art annimmt.
    Nicht benötigte Dienste abzuschalten ist der einfachste Weg,
    unnötige Zugänge zu schließen.
connection_info_port=Port
connection_info_port-status=TCP-Port für eingehende Verbindungen
connection_info_port-help=
    # Port

    TCP-Port des Dienstes. Üblich sind 23 für Telnet und 22 für SSH.
    Für Ports unter 1024 benötigt das Board eine entsprechende Berechtigung.
connection_info_address=Adresse
connection_info_address-status=Lokale Adresse, an der der Dienst Verbindungen annimmt
connection_info_address-help=
    # Adresse

    Lokale Bindeadresse des Dienstes. Eine Adresse für alle Schnittstellen
    akzeptiert Verbindungen aus allen Netzen. Eine einzelne Adresse
    beschränkt den Dienst auf das betreffende Netzwerk.
connection_info_display_file=Anzeigedatei
connection_info_display_file-status=Anzeige für Benutzer dieses Dienstes
connection_info_display_file-help=
    # Anzeigedatei

    Bildschirm vor der üblichen Anmeldung für Benutzer dieses Dienstes.
    Hier gehören Hinweise hin, die nur diese Verbindungsart betreffen.

# ICBSetup -> Ereignisse
event_setup_title=Ereignisse
event_enabled_for_expedited_label=Für vorgezogene Ereignisse (EXPEDITED):
event_enabled=Zeitgesteuerte Ereignisse aktiv
event_enabled-status=Zeitgesteuerte Ereignisse aktivieren
event_enabled-help=
    # Zeitgesteuerte Ereignisse aktiv

    Legt fest, ob das Board zeitgesteuerte Ereignisse ausführt.
    Ausgeschaltet bleiben die Einstellungen erhalten, aber nichts wird eingeplant.
event_file=Ereignisdatei
event_file-status=Name/Pfad der Liste zeitgesteuerter Ereignisse
event_file-help=TOML-Datei mit den zeitgesteuerten Ereignissen. F2 öffnet den Editor und legt die Datei bei Bedarf an.
event_suspend_minutes=Aktivitätsstopp Minuten vorher
event_suspend_minutes-status=Minuten vor dem Ereignis alle Aktivitäten aussetzen
event_suspend_minutes-help=
    # Aktivitätsstopp Minuten vorher

    So lange vor dem Ereignis verhindert das Board Aktivitäten,
    die bei dessen Beginn noch laufen könnten. Die Benutzerzeit
    wird verkürzt, damit das Ereignis nicht warten muss.
event_disallow_uploads=Uploads vor Ereignis sperren
event_disallow_uploads-status=Uploads vor dem Ereignis sperren
event_disallow_uploads-help=
    # Uploads vor Ereignis sperren

    Schaltet Uploads kurz vor dem Ereignis ab, damit keine Übertragung
    mehr läuft, wenn das Board herunterfahren muss.
event_minutes_uploads_disallowed=Uploadstopp Minuten vorher
event_minutes_uploads_disallowed-status=Minuten vor dem Ereignis keine Uploads mehr annehmen
event_minutes_uploads_disallowed-help=
    # Uploadstopp Minuten vorher

    So viele Minuten vor dem Ereignis werden Uploads nicht mehr angenommen.
    Der Vorlauf sollte für den größten üblichen Upload ausreichen.

# ICBSetup -> Abrechnung
accounting_config_title=Abrechnung
accounting_enabled=Abrechnung aktivieren
accounting_enabled-status=Abrechnungsfunktionen aktivieren
accounting_enabled-help=
    # Abrechnung aktivieren

    Aktiviert das Guthabenkonto jedes Benutzers. Gebühren und Vergütungen
    stammen aus der Tarifdatei. Eine Sicherheitsstufe nimmt nur teil,
    wenn ihr Eintrag in der PWRD-Datei das Konto aktiviert.
accounting_use_money=Geld statt Punkte anzeigen
accounting_use_money-status=Geldbeträge statt Guthabenpunkten anzeigen
accounting_use_money-help=
    # Geld statt Punkte anzeigen

    Zeigt Guthaben und Gebühren mit einem Währungssymbol statt als
    einfache Punkte an. Geeignet für Boards, die tatsächlich Geld berechnen.
accounting_concurrent_tracking=Gebühren laufend abrechnen
accounting_concurrent_tracking-status=Gebühren während der Sitzung abrechnen
accounting_concurrent_tracking-help=
    # Gebühren laufend abrechnen

    Aktualisiert das Guthaben während der Sitzung statt erst am Ende.
    So kann ein Benutzer innerhalb eines Anrufs sein Konto nicht überziehen.
accounting_ignore_empty_sec_level=Stufe bei leerem Konto ignorieren
accounting_ignore_empty_sec_level-status=Sicherheitsstufe bei leerem Guthaben beibehalten
accounting_ignore_empty_sec_level-help=
    # Stufe bei leerem Konto ignorieren

    Normalerweise fällt ein Benutzer bei leerem Konto auf die dafür im
    Datensatz festgelegte Stufe zurück. Diese Option behält stattdessen die normale Stufe bei.
accounting_peak_usage_start=Beginn der Hauptzeit
accounting_peak_usage_start-status=Beginn der Hauptnutzungszeit
accounting_peak_usage_start-help=
    # Beginn der Hauptzeit

    Beginn der Hauptnutzungszeit im 24-Stunden-Format. Minuten innerhalb
    dieses Zeitraums werden zum Hauptzeittarif statt zum Normaltarif berechnet.
accounting_peak_usage_end=Ende der Hauptzeit
accounting_peak_usage_end-status=Ende der Hauptnutzungszeit
accounting_peak_usage_end-help=
    # Ende der Hauptzeit

    Ende der Hauptnutzungszeit im 24-Stunden-Format.
accounting_peak_days_of_week=Hauptzeit-Wochentage
accounting_peak_days_of_week-status=Wochentage mit Hauptzeittarif
accounting_peak_days_of_week-help=
    # Hauptzeit-Wochentage

    Tage, an denen der Hauptzeittarif gilt. Ohne ausgewählten Tag
    ist der Hauptzeittarif vollständig ausgeschaltet.
accounting_peak_holiday_list_file=Feiertagsliste
accounting_peak_holiday_list_file-status=Name/Pfad der Feiertagsliste für den Hauptzeittarif
accounting_peak_holiday_list_file-help=
    # Feiertagsliste

    Daten, an denen der Hauptzeittarif ausgesetzt wird. So gilt an
    Feiertagen der günstigere Tarif, auch wenn sie auf einen Hauptzeittag fallen.
accounting_cfg_file=Tarifdatei
accounting_cfg_file-status=Name/Pfad der Abrechnungskonfiguration
accounting_cfg_file-help=
    # Tarifdatei

    Gebühren und Vergütungen für Onlinezeit, Nachrichten, Uploads
    und Downloads. F2 öffnet die Tarifbearbeitung.
accounting_tracking_file=Buchungsprotokoll
accounting_tracking_file-status=Name/Pfad des Buchungsprotokolls
accounting_tracking_file-help=
    # Buchungsprotokoll

    Erfasst jede Buchung auf Benutzerkonten, damit sich ein
    Kontostand später nachvollziehen lässt.
accounting_info_file=Kontoinformation
accounting_info_file-status=Name/Pfad der Kontoinformationsdatei
accounting_info_file-help=
    # Kontoinformation

    Wird bei der Anmeldung angezeigt und informiert über den Kontostand.
accounting_warning_file=Kontowarnung
accounting_warning_file-status=Name/Pfad der Kontowarndatei
accounting_warning_file-help=
    # Kontowarnung

    Wird bei der Anmeldung angezeigt, sobald das Guthaben die Warnschwelle
    erreicht. So kann rechtzeitig vor einem leeren Konto aufgeladen werden.
accounting_logoff_file=Abmeldeabrechnung
accounting_logoff_file-status=Name/Pfad der Abrechnungsdatei zur Abmeldung
accounting_logoff_file-help=
    # Abmeldeabrechnung

    Wird beim Abmelden angezeigt. Hier lassen sich Sitzungskosten
    und verbleibendes Guthaben mitteilen.

# Konferenzeditor
conf_name=Name (#{ $number })
conf_public_conf=Öffentliche Konferenz
conf_public_conf-status=Öffentliche Konferenz
conf_public_conf-help=
    # Öffentliche Konferenz

    Eine öffentliche Konferenz kann jeder mit den unten geforderten Rechten
    betreten. Eine private ist unabhängig von der Stufe nur registrierten Benutzern zugänglich.
conf_req_sec_if_pub=Zugriff bei öffentlicher Konferenz
conf_req_sec_if_pub-status=Benötigte Rechte für eine öffentliche Konferenz
conf_req_sec_if_pub-help=
    # Zugriff bei öffentlicher Konferenz

    Für den Beitritt benötigte Rechte, solange die Konferenz öffentlich ist.
    Möglich sind eine einfache Stufe oder ein Ausdruck, der etwa eine Gruppe zulässt.
conf_pw_join_priv=Passwort für private Konferenz
conf_pw_join_priv-status=Passwort zum Beitritt zur privaten Konferenz
conf_pw_join_priv-help=
    # Passwort für private Konferenz

    Dieses Passwort öffnet die private Konferenz für jeden, der es kennt.
    Damit entfällt die manuelle Registrierung jedes Benutzers.
conf_user_menu=Benutzermenü
conf_user_menu-status=Name/Pfad des Benutzermenüs
conf_user_menu-help=
    # Benutzermenü

    Menü für normale Benutzer dieser Konferenz.
    Leer lassen, um das Boardmenü zu verwenden.
conf_sysop_menu=Sysop-Menü
conf_sysop_menu-status=Name/Pfad des Sysop-Menüs
conf_sysop_menu-help=
    # Sysop-Menü

    Menü für Benutzer ab der Sysop-Stufe. Üblicherweise enthält es
    das Benutzermenü und zusätzlich die Sysop-Befehle.
conf_news_file=NEWS-Datei
conf_news_file-status=Name/Pfad der NEWS-Datei
conf_news_file-help=
    # NEWS-Datei

    Neuigkeiten dieser Konferenz. Sie erscheinen beim Neuigkeitenbefehl
    und beim Beitritt entsprechend der NEWS-Einstellung des Boards.
conf_intro_file=INTRO-Datei
conf_intro_file-status=Name/Pfad der Konferenz-INTRO-Datei
conf_intro_file-help=
    # INTRO-Datei

    Wird beim Konferenzbeitritt angezeigt. Hier lassen sich Zweck
    der Konferenz und Erwartungen an die Teilnehmer erklären.
conf_attach_loc=Anhangverzeichnis
conf_attach_loc-status=Verzeichnis für Dateianhänge
conf_attach_loc-help=
    # Anhangverzeichnis

    Hier werden Dateianhänge von Nachrichten dieser Konferenz gespeichert.
conf_cmd_lst_file=Konferenz-CMD.LST
conf_cmd_lst_file-status=CMD.LST-Datei der Konferenz
conf_cmd_lst_file-help=
    # Konferenz-CMD.LST

    Zusätzliche Befehle nur für diese Konferenz,
    ergänzend zur boardweiten Befehlsliste.
conf_sort_loc_label={"              "}Sort. Name/Pfad METADATA            Uploadverzeichnis
conf_upload_sort_header=Sort.
conf_upload_metadata_header=Name/Pfad METADATA
conf_upload_location_header=Uploadverzeichnis
conf_menu_display_header=Menüanzeige
conf_menu_list_header=Pfad/Name der Listendatei
conf_pub_upld=Öffentl. Upload
conf_pub_upld-status=Öffentlicher Upload
conf_pub_upld-help=
    # Öffentlicher Upload

    Hier landen Uploads, sobald alle sie sehen dürfen.
    Aus diesem Verzeichnis entsteht die Liste neuer Dateien.
conf_priv_upld=Privater Upload
conf_priv_upld-status=Privater Upload
conf_priv_upld-help=
    # Privater Upload

    Hier warten Uploads, solange nur der Sysop sie sehen darf.
    Zum Anzeigen ist die Sysop-Stufe für private Uploads nötig.
conf_menu_path_label={"              "}Menüanzeige                    Pfad/Name der Listendatei
conf_doors=Doors
conf_doors-status=Doors
conf_doors-help=
    # Doors

    Liste der Door-Programme dieser Konferenz. Mit F2 bearbeiten.
conf_bulletins=Bulletins
conf_bulletins-status=Bulletins
conf_bulletins-help=
    # Bulletins

    Liste der Bulletins dieser Konferenz. Mit F2 bearbeiten.
conf_surveys=Umfragen
conf_surveys-status=Umfragen
conf_surveys-help=
    # Umfragen

    Liste der hier angebotenen Umfragen. Mit F2 bearbeiten.
conf_directories=Verzeichnisse
conf_directories-status=Verzeichnisse
conf_directories-help=
    # Verzeichnisse

    Dateiverzeichnisse dieser Konferenz, die der Befehl F auflistet.
    Mit F2 bearbeiten.
conf_areas=Nachrichtenbereiche
conf_areas-status=Nachrichtenbereiche
conf_areas-help=
    # Nachrichtenbereiche

    Nachrichtenbereiche dieser Konferenz. Anders als bei PCBoard darf
    eine Konferenz mehrere Nachrichtenbereiche statt nur einer Basis
    enthalten. Mit F2 bearbeiten.
conf_auto_rejon=Automatisch wieder beitreten
conf_auto_rejon-status=Dieser Konferenz automatisch wieder beitreten
conf_auto_rejon-help=
    # Automatisch wieder beitreten

    Führt Benutzer bei der Anmeldung direkt in diese Konferenz
    statt in die Hauptkonferenz.
conf_add_conf_sec=Zusätzliche Sicherheitsstufe
conf_add_conf_sec-status=Zusätzliche Sicherheitsstufe in dieser Konferenz
conf_add_conf_sec-help=
    # Zusätzliche Sicherheitsstufe

    Wird innerhalb dieser Konferenz zur Sicherheitsstufe des Benutzers addiert.
    So kann ein vertrauenswürdiger Bereich Rechte gewähren, die anderswo fehlen.
conf_allow_view_conf_members=Mitgliederliste erlauben
conf_allow_view_conf_members-status=Anzeigen der Konferenzmitglieder erlauben
conf_allow_view_conf_members-help=
    # Mitgliederliste erlauben

    Erlaubt Benutzern aufzulisten, wer hier noch registriert ist.
    Ausschalten, wenn bereits die Mitgliedschaft vertraulich bleiben soll.
conf_add_conference_time=Zusätzliche Konferenzzeit
conf_add_conference_time-status=Zusätzliche Zeit in dieser Konferenz
conf_add_conference_time-help=
    # Zusätzliche Konferenzzeit

    Zusätzliche Minuten innerhalb dieser Konferenz. So kann etwa ein
    Supportbereich länger genutzt werden als der Rest des Boards.
conf_private_uploads=Alle Uploads privat
conf_private_uploads-status=Alle Uploads privat halten
conf_private_uploads-help=
    # Alle Uploads privat

    Behält jeden Upload im privaten Verzeichnis, bis der Sysop ihn verschiebt.
    So wird nichts ungeprüft für andere Benutzer sichtbar.
conf_private_messages=Alle Nachrichten privat
conf_private_messages-status=Alle Nachrichten privat machen
conf_private_messages-help=
    # Alle Nachrichten privat

    Alle hier geschriebenen Nachrichten sind privat adressiert,
    sodass nur Absender und Empfänger sie lesen können.
conf_disallow_private_msgs=Private Nachrichten verbieten
conf_disallow_private_msgs-status=Private Nachrichten verbieten
conf_disallow_private_msgs-help=
    # Private Nachrichten verbieten

    Erzwingt öffentliche Nachrichten, wie es eine Echomail-Konferenz
    ohne private Post benötigt.
conf_sec_attachments=Stufe für Dateianhänge
conf_sec_attachments-status=Stufe zum Speichern von Dateianhängen
conf_sec_attachments-help=
    # Stufe für Dateianhänge

    Benötigte Rechte, um hier eine Datei an eine Nachricht anzuhängen.
conf_show_intro_in_scan=INTRO bei 'R A' anzeigen
conf_show_intro_in_scan-status=INTRO beim Lesen mit 'R A' anzeigen
conf_show_intro_in_scan-help=
    # INTRO bei 'R A' anzeigen

    Zeigt die Konferenzeinleitung beim Lesen aller Konferenzen an.
    So bleibt erkennbar, aus welcher Konferenz die Nachrichten stammen.
conf_sec_write_message=Stufe zum Nachrichtenschreiben
conf_sec_write_message-status=Stufe zum Schreiben einer Nachricht
conf_sec_write_message-help=
    # Stufe zum Nachrichtenschreiben

    Hier benötigte Schreibrechte, zusätzlich zur boardweiten Stufe für E.
conf_sec_carbon_copy=Stufe für Nachrichtenkopien
conf_sec_carbon_copy-status=Stufe zum Senden an eine Empfängerliste
conf_sec_carbon_copy-help=
    # Stufe für Nachrichtenkopien

    Benötigte Rechte, um hier eine Nachricht an eine Empfängerliste zu senden.
conf_carbon_list_limit=Maximale Kopienempfänger
conf_carbon_list_limit-status=Maximale Anzahl der Kopienempfänger
conf_carbon_list_limit-help=
    # Maximale Kopienempfänger

    Maximale Empfängerzahl einer Nachricht mit Kopien. Verhindert,
    dass eine einzelne Nachricht gleichzeitig an das ganze Board geht.
conf_allow_aliases=Aliasse erlauben
conf_allow_aliases-status=Verwendung von Aliasnamen erlauben
conf_allow_aliases-help=
    # Aliasse erlauben

    Legt fest, ob Benutzer hier unter ihrem Alias auftreten.
    Ausschalten, wenn Nachrichten echte Namen tragen sollen,
    etwa in einer Supportkonferenz.
conf_charge_time=Gebühr pro Minute
conf_charge_time-status=Gebühr pro Minute
conf_charge_time-help=
    # Gebühr pro Minute

    Wird pro Minute in dieser Konferenz zusätzlich zum Boardtarif berechnet.
    Wirkt nur bei eingeschalteter Abrechnung.
conf_charge_msg_read=Gebühr pro gelesener Nachricht
conf_charge_msg_read-status=Gebühr pro gelesener Nachricht
conf_charge_msg_read-help=
    # Gebühr pro gelesener Nachricht

    Gebühr je hier gelesener Nachricht. Damit kann ein aufwendiger
    Bereich mehr kosten als der Rest des Boards.
conf_charge_msg_write=Gebühr pro geschriebener Nachricht
conf_charge_msg_write-status=Gebühr pro geschriebener Nachricht
conf_charge_msg_write-help=
    # Gebühr pro geschriebener Nachricht

    Gebühr je hier geschriebener Nachricht. Negative Werte vergüten
    den Benutzer und belohnen Beiträge.
conf_is_read_only=Konferenz schreibschützen
conf_is_read_only-status=Konferenz nur zum Lesen freigeben
conf_is_read_only-help=
    # Konferenz schreibschützen

    Benutzer dürfen hier lesen, aber nicht schreiben. Geeignet für
    Ankündigungsbereiche, deren Inhalte von anderswo kommen.
conf_echo_mail_in_conference=Echomail in Konferenz
conf_echo_mail_in_conference-status=Konferenz für Echomail kennzeichnen
conf_echo_mail_in_conference-help=
    # Echomail in Konferenz

    Kennzeichnet die Konferenz als Netzpostbereich. Hier geschriebene
    Nachrichten werden für andere Systeme verpackt, statt lokal zu bleiben.
conf_list_title=Konferenzmenü

# Tarifeditor
accounting_title=Abrechnungstarife
accounting_start_balance=Startguthaben neuer Benutzer
accounting_start_balance-status=Startguthaben neuer Benutzer
accounting_start_balance-help=
    # Startguthaben neuer Benutzer

    Guthaben bei Eröffnung eines Benutzerkontos. Damit können neue
    Benutzer sich umsehen, bevor sie etwas bezahlt haben.
accounting_warning_level=Guthaben-Warnschwelle
accounting_warning_level-status=Warnschwelle für niedriges Guthaben
accounting_warning_level-help=
    # Guthaben-Warnschwelle

    Ab diesem Kontostand wird bei der Anmeldung vor knappem Guthaben gewarnt.
accounting_charges_label=Gebühren:
accounting_per_logon=Pro Anmeldung
accounting_per_logon-status=Pro Anmeldung
accounting_per_logon-help=
    # Pro Anmeldung

    Einmalige Gebühr je erfolgreicher Anmeldung. Ein negativer Wert
    vergütet den Benutzer und belohnt damit den Anruf.
accounting_per_minute=Pro Onlineminute
accounting_per_minute-status=Pro Onlineminute
accounting_per_minute-help=
    # Pro Onlineminute

    Gebühr je Onlineminute außerhalb der Hauptnutzungszeiten.
accounting_per_minute_peak=Pro Minute zur Hauptzeit
accounting_per_minute_peak-status=Pro Onlineminute zur Hauptzeit
accounting_per_minute_peak-help=
    # Pro Minute zur Hauptzeit

    Gebühr je Onlineminute innerhalb der Hauptzeit an den dafür ausgewählten Tagen.
accounting_per_minute_grpChat=Pro Minute im Gruppenchat (zusätzlich)
accounting_per_minute_grpChat-status=Zusätzliche Gebühr pro Minute im Gruppenchat
accounting_per_minute_grpChat-help=
    # Pro Minute im Gruppenchat

    Gebühr je Minute im Gruppenchat. Sie wird zur normalen
    Minutengebühr addiert und ersetzt diese nicht.
accounting_per_message_read=Pro gelesener Nachricht
accounting_per_message_read-status=Pro gelesener Nachricht
accounting_per_message_read-help=
    # Pro gelesener Nachricht

    Gebühr je online gelesener Nachricht. Für Nachrichten in einem
    heruntergeladenen Paket gilt stattdessen der Mitschnitttarif.
accounting_per_message_captured=Pro mitgenommener Nachricht (QWK/c/d/z)
accounting_per_message_captured-status=Pro mitgenommener Nachricht (QWK/c/d/z)
accounting_per_message_captured-help=
    # Pro mitgenommener Nachricht

    Gebühr je Nachricht, die als Mitschnitt oder in einem QWK-Paket
    mitgenommen wird, statt sie online zu lesen.
accounting_per_message_written=Pro geschriebener Nachricht
accounting_per_message_written-status=Pro geschriebener Nachricht
accounting_per_message_written-help=
    # Pro geschriebener Nachricht

    Gebühr je lokal geschriebener öffentlicher Nachricht. Negative Werte
    belohnen das Schreiben und können den Nachrichtenaustausch fördern.
accounting_per_message_written_echoed=Pro geschriebener Echomail
accounting_per_message_written_echoed-status=Pro geschriebener Echomail-Nachricht
accounting_per_message_written_echoed-help=
    # Pro geschriebener Echomail

    Gebühr für Nachrichten, die das Board über ein Netzwerk verlassen.
    Getrennt vom Lokaltarif, weil Echomail dem Board Kosten verursacht.
accounting_per_message_written_private=Pro geschriebener Privatnachricht
accounting_per_message_written_private-status=Pro geschriebener Privatnachricht
accounting_per_message_written_private-help=
    # Pro geschriebener Privatnachricht

    Gebühr je privater Nachricht. Sie kann vom Tarif für öffentliche,
    allen zugängliche Nachrichten abweichen.
accounting_per_file_downloaded=Pro heruntergeladener Datei
accounting_per_file_downloaded-status=Pro heruntergeladener Datei
accounting_per_file_downloaded-help=
    # Pro heruntergeladener Datei

    Gebühr je heruntergeladener Datei, unabhängig von ihrer Größe.
accounting_per_file_bytes_downloaded=Pro 1 KB Download
accounting_per_file_bytes_downloaded-status=Pro 1 KB Download
accounting_per_file_bytes_downloaded-help=
    # Pro 1 KB Download

    Gebühr je heruntergeladenem Kilobyte. Große Dateien kosten dadurch mehr als kleine.
accounting_payback_label=Vergütungen:
accounting_payback_per_file=Pro hochgeladener Datei
accounting_payback_per_file-status=Pro hochgeladener Datei
accounting_payback_per_file-help=
    # Pro hochgeladener Datei

    Vergütung je hochgeladener Datei. So verdienen Uploads das
    Guthaben, das Downloads verbrauchen.
accounting_payback_per_file_bytes=Pro 1 KB Upload
accounting_payback_per_file_bytes-status=Pro 1 KB Upload
accounting_payback_per_file_bytes-help=
    # Pro 1 KB Upload

    Vergütung je hochgeladenem Kilobyte. Größere Beiträge bringen damit mehr Guthaben.

# IcyBoard System Manager
icbsm_main_menu_title=Hauptmenü
icb_sysmanager_main_title=Benutzerdateiverwaltung
icbsm_main_users=Benutzerdateiverwaltung
icbsm_main_directory=Verzeichnisverwaltung
icbsm_dir_colors=DIR-Dateifarben anpassen
icb_sysmanager_main_edit_users=Benutzerdatei bearbeiten
icb_sysmanager_main_edit_groups=Gruppen bearbeiten
icbsm_menu_edit_users=Benutzerdatei bearbeiten
icbsm_menu_sort=Benutzerdatei sortieren
icbsm_menu_pack=Benutzerdatei packen
icbsm_menu_adjust_security=Sicherheitsstufen anpassen
icbsm_menu_insert_conf=Konferenzregistrierungen hinzufügen
icbsm_menu_remove_conf=Konferenzregistrierungen entfernen
icbsm_menu_move_conf=Benutzer zwischen Konferenzen verschieben
icbsm_menu_expiration=Ablaufdaten anpassen
icbsm_menu_phones=Telefonformate vereinheitlichen
icbsm_menu_undo=Rückgängig (Sicherung wiederherstellen)
icbsm_menu_groups=Gruppen bearbeiten
icbsm_pack_title=Benutzerdatei packen
icbsm_adjust_security_title=Sicherheitsstufen anpassen
icbsm_phones_title=Telefonformate vereinheitlichen
icbsm_undo_title=Rückgängig (Sicherung wiederherstellen)
icbsm_sec_by_ranges=Nach Bereichen anpassen
icbsm_sec_by_ranges_title=Sicherheitsstufe nach Bereich anpassen
icbsm_sec_by_ranges_expired_title=Ablaufstufe nach Bereich anpassen
icbsm_sec_by_ranges_expired=Nach Bereichen anpassen (Ablaufstufe)
icbsm_sec_by_file_ratio=Nach Upload-/Download-Dateiverhältnis
icbsm_sec_by_byte_ratio=Nach Upload-/Download-Byteverhältnis
icbsm_sec_by_uploads=Nach Anzahl der Uploads
icbsm_sec_by_downloads=Nach Anzahl der Downloads
icbsm_sec_table_file_ratio=Dateiverhältnistabelle erstellen
icbsm_sec_table_byte_ratio=Byteverhältnistabelle erstellen
icbsm_sec_table_uploads=Uploadtabelle erstellen
icbsm_sec_table_downloads=Downloadtabelle erstellen
icbsm_sec_copy_expired=Auf Ablaufstufe setzen
icbsm_sec_init_counters=Upload-/Downloadzähler initialisieren
icbsm_table_title_file_ratio=Upload-/Download-Dateiverhältnistabelle
icbsm_table_title_byte_ratio=Upload-/Download-Byteverhältnistabelle
icbsm_table_title_uploads=Uploadtabelle bearbeiten
icbsm_table_title_downloads=Downloadtabelle bearbeiten
icbsm_table_column_file_ratio=Verhältnis
icbsm_table_column_byte_ratio=Verhältnis
icbsm_table_column_uploads=Uploads
icbsm_table_column_downloads=Downloads
icbsm_table_security=Stufe
icbsm_table_help_title_file_ratio=Sicherheitsstufen nach Upload-/Downloadverhältnis
icbsm_table_help_title_byte_ratio=Sicherheitsstufen nach Upload-/Downloadverhältnis
icbsm_table_help_title_uploads=Sicherheitsstufen nach Uploadanzahl
icbsm_table_help_title_downloads=Sicherheitsstufen nach Downloadanzahl
icbsm_table_help_file_ratio =
    Jedem gewünschten Upload-/Downloadverhältnis eine Sicherheitsstufe zuordnen. Das Verhältnis ist die Anzahl der Uploads geteilt durch die Anzahl der Downloads.
    {" "}
    Beispiele:
    {" "}
      0.1  bedeutet   1 Upload auf 10 Downloads
      1.0  bedeutet   gleich viele Uploads und Downloads
      5.0  bedeutet   5 Uploads auf 1 Download
    {" "}
    HINWEIS: Unterhalb des kleinsten Tabellenschritts bleibt die bisherige Sicherheitsstufe erhalten.
icbsm_table_help_byte_ratio =
    Jedem gewünschten Upload-/Download-Byteverhältnis eine Sicherheitsstufe zuordnen. Das Verhältnis ist die Anzahl hochgeladener Bytes geteilt durch die Anzahl heruntergeladener Bytes.
    {" "}
    Beispiele:
    {" "}
      0.1  bedeutet   1 Uploadbyte auf 10 Downloadbytes
      1.0  bedeutet   gleich viele Upload- und Downloadbytes
      5.0  bedeutet   5 Uploadbytes auf 1 Downloadbyte
    {" "}
    HINWEIS: Unterhalb des kleinsten Tabellenschritts bleibt die bisherige Sicherheitsstufe erhalten.
icbsm_table_help_uploads =
    Jeder gewünschten Uploadanzahl eine Sicherheitsstufe zuordnen. Wer diese Anzahl erreicht, erhält die zugehörige Stufe, unabhängig davon, ob sie höher oder niedriger als die bisherige ist.
    {" "}
    Beispiel:       Uploads   Stufe
    {" "}
      Angenommen,         0      10
      diese Tabelle      10      25
      wird verwendet:    20      30
                         30      35
    {" "}
    Ein Benutzer mit 10 bis 19 Uploads und Stufe 20 wird auf 25 angehoben. Unterhalb des kleinsten Schritts bleibt die Stufe unverändert.
icbsm_table_help_downloads =
    Jeder gewünschten Downloadanzahl eine Sicherheitsstufe zuordnen. Wer diese Anzahl erreicht, erhält die zugehörige Stufe, unabhängig davon, ob sie höher oder niedriger als die bisherige ist.
    {" "}
    Beispiel:     Downloads   Stufe
    {" "}
      Angenommen,         0      35
      diese Tabelle      10      25
      wird verwendet:    20      20
                         30      15
    {" "}
    Ein Benutzer mit 20 bis 29 Downloads und Stufe 15 wird auf 20 angehoben. Unterhalb des kleinsten Schritts bleibt die Stufe unverändert.
icbsm_table_empty=Die Tabelle enthält noch keine Schritte. Zuerst erstellen.
icbsm_table_hint=Ein Schritt mit Sicherheitsstufe 0 wird nicht übernommen.
icbsm_table_keys=ESC=Ende   PGDN=Tabelle speichern   Pfeile=Bewegen
icbsm_table_saved=Die Tabelle wurde gespeichert.
icbsm_counters_title=Upload-/Downloadzähler initialisieren
icbsm_counters_option1=1) Felder ANGLEICHEN (an Downloadfeld)
icbsm_counters_option2=2) Felder ANGLEICHEN (an Uploadfeld)
icbsm_counters_option3=3) Upload- und Downloadfelder auf NULL setzen
icbsm_counters_option4=4) Beide BYTE-Zähler nach Upload:Download-DATEIverhältnis setzen
icbsm_counters_choose=Option wählen (1, 2, 3 oder 4 von oben)
icbsm_counters_files=Upload-/Download-DATEIzähler anpassen
icbsm_counters_bytes=Upload-/Download-BYTEzähler anpassen
icbsm_apply_table_question=Sicherheitsstufen nach den { $count } Tabellenschritten anpassen?
icbsm_question_keys=PGDN=Ja   ESC=Abbrechen
icbsm_are_you_sure=Sind Sie sicher?
icbsm_sort_options_title=Sortieroptionen
icbsm_sort_single_title=Nach einem Feld
icbsm_sort_multiple_title=Nach mehreren Feldern
icbsm_sort_run_title=Benutzerdatei sortieren
icbsm_sort_name=Name
icbsm_sort_password=Passwort
icbsm_sort_bus_phone=Geschäfts-/Datentelefon
icbsm_sort_home_phone=Privat-/Sprachtelefon
icbsm_sort_registration=Registrierungsablauf
icbsm_sort_comment1=Kommentar 1
icbsm_sort_comment2=Kommentar 2
icbsm_sort_city=Wohnort
icbsm_sort_security_name=Sicherheitsstufe, dann Name
icbsm_sort_times_on_name=Anrufanzahl, dann Name
icbsm_sort_dnld_name=Downloadanzahl, dann Name
icbsm_sort_upld_name=Uploadanzahl, dann Name
icbsm_sort_file_ratio_name=Upload:Download-Dateiverhältnis, dann Name
icbsm_sort_dnld_bytes_name=Downloadbytes, dann Name
icbsm_sort_upld_bytes_name=Uploadbytes, dann Name
icbsm_sort_byte_ratio_name=Upload:Download-Byteverhältnis, dann Name
icbsm_sort_field=Benutzerdatei nach { $field } sortieren
icbsm_sort_reverse=Umgekehrte Reihenfolge: { $value }
icbsm_sort_done={ $count } Datensätze verschoben.
icbsm_yes=Ja
icbsm_no=Nein
icbsm_sort_keys=R Reihenfolge umkehren, PGDN Start, ESC Abbrechen
icbsm_menu_keys=Pfeile bewegen die Auswahl, ENTER wählt aus, ESC beendet
icbsm_min_security=Benutzer ändern mit Sicherheitsstufe größer oder gleich
icbsm_max_security=und Sicherheitsstufe kleiner oder gleich
icbsm_use_expired_level=ABLAUFSTUFE als Sicherheitskriterium verwenden
icbsm_pack_removal_group=Kriterien zum Entfernen von Benutzerdatensätzen
icbsm_pack_keep_group=Kriterien zum Behalten von Benutzerdatensätzen
icbsm_remove_deleted_or_locked=Gelöschte oder GESPERRTE Benutzer entfernen
icbsm_inactive_days=Benutzer nach XXXX Tagen ohne Anmeldung entfernen
icbsm_inactive_days-status=9999 ignoriert den letzten Anruf
icbsm_last_on_since=Benutzer ohne Anmeldung seit diesem Datum entfernen
icbsm_expired_before=Benutzer mit Registrierungsablauf vor diesem Datum entfernen
icbsm_date_off-status=01-01-80 schaltet dieses Kriterium aus
icbsm_keep_security=Benutzer mit Sicherheitsstufe größer oder gleich behalten
icbsm_keep_security-status=0 behält niemanden aufgrund seiner Sicherheitsstufe
icbsm_keep_locked_out=GESPERRTE Benutzer behalten
icbsm_new_level=Neue Sicherheitsstufe
icbsm_write_expired_level=Stattdessen die Ablaufstufe ändern
icbsm_copy_expired_level=Auf Ablaufstufe setzen
icbsm_copy_expired_level-status=Neue Stufe stattdessen aus dem jeweiligen Datensatz übernehmen
icbsm_expiration_title=Ablaufdatum ändern
icbsm_expiration_range_group=Sicherheitsstufenbereich
icbsm_expiration_change_group=Ablaufdatum ändern auf:
icbsm_exp_min_security=Ablaufdatum ändern bei Stufe größer oder gleich
icbsm_exp_max_security=Ablaufdatum ändern bei Stufe kleiner oder gleich
icbsm_expiration_date=Neues Ablaufdatum (01-01-80 wird ignoriert)
icbsm_add_days=Aktuelles Datum im Datensatz plus XXXX Tage
icbsm_conf_insert_title=Konferenzregistrierungen gruppenweise hinzufügen
icbsm_conf_remove_title=Konferenzregistrierungen gruppenweise entfernen
icbsm_conf_move_title=Benutzer zwischen Konferenzen verschieben
icbsm_conf_first_insert=Erste hinzuzufügende Konferenznummer
icbsm_conf_last_insert=Letzte hinzuzufügende Konferenznummer
icbsm_conf_first_remove=Erste zu entfernende Konferenznummer
icbsm_conf_last_remove=Letzte zu entfernende Konferenznummer
icbsm_conf_min_security=Benutzer ändern mit Sicherheitsstufe größer oder gleich
icbsm_conf_max_security=und kleiner oder gleich
icbsm_conf_from=Benutzer aus welcher Konferenz ENTFERNEN
icbsm_conf_to=Zu welcher Konferenz HINZUFÜGEN
icbsm_move_min_security=Benutzer einbeziehen mit Sicherheitsstufe GRÖSSER ODER GLEICH
icbsm_move_max_security=Benutzer einbeziehen mit Sicherheitsstufe KLEINER ODER GLEICH
icbsm_flag_registered=Normal zugängliche Konferenzen anpassen
icbsm_flag_expired=Bei abgelaufenem Abonnement zugängliche Konferenzen anpassen
icbsm_flag_selected=Für die Nachrichtensuche gewählte Konferenzen anpassen
icbsm_flag_sysop=Konferenzen mit Sysop-Rechten bei Beitritt anpassen
icbsm_reset_lastread=Lesezeiger des Benutzers in diesen Konferenzen auf null setzen
icbsm_flag_net_status=Konferenzen mit Netzstatus des Benutzers anpassen
icbsm_move_flag_registered=Jederzeit zugängliche Konferenzen anpassen
icbsm_move_flag_expired=Bei abgelaufenem Abonnement zugängliche Konferenzen anpassen
icbsm_move_flag_selected=Konferenzauswahl für die Nachrichtensuche anpassen
icbsm_move_flag_sysop=Konferenzen mit Sysop-Rechten bei Beitritt anpassen
icbsm_move_lastread=Lesezeiger in die neue Konferenz übernehmen
icbsm_move_last_conference=Kennzeichen „Letzte Konferenz“ setzen
icbsm_criteria_keys=PGDN Start, ESC Abbrechen
icbsm_preview_keys=ENTER Ausführen, ESC Zurück
icbsm_done_keys=Beliebige Taste drücken
icbsm_undo_keys=ENTER Wiederherstellen, ESC Abbrechen
icbsm_preview_count={ $count } Benutzer ausgewählt
icbsm_preview_more=… und { $count } weitere
icbsm_preview_pack_warning=Diese Datensätze werden entfernt. Zuvor wird eine Sicherung erstellt.
icbsm_done_count={ $changed } von { $matched } Benutzern geändert
icbsm_done_backup_hint=Die vorige Benutzerdatei wurde aufbewahrt. Wiederherstellung über das Hauptmenü.
icbsm_backup_failed=Sicherung fehlgeschlagen; nichts wurde geändert: { $error }
icbsm_save_failed=Benutzerdatei konnte nicht gespeichert werden: { $error }
icbsm_undo_prompt=Benutzerdatei aus der Sicherung vom { $date } wiederherstellen?
icbsm_undo_no_backup=Keine Sicherung zum Wiederherstellen vorhanden.
icbsm_undo_done=Die Benutzerdatei wurde wiederhergestellt.
icbsm_undo_failed=Benutzerdatei konnte nicht wiederhergestellt werden: { $error }
icbsm_board_in_use=Ein anderes Werkzeug bearbeitet dieses Board. Dieses schließen und erneut starten.
icbsm_record_one_protected=Datensatz #1 ist der Sysop-Datensatz und kann nicht entfernt werden.
icbsm_list_sort_record=Datensatz
icbsm_list_sort_name=Name
icbsm_list_sort_security=Sicherheitsstufe
icbsm_list_sort_last_on=Letzte Anmeldung
icbsm_user_list_keys=F2 Speichern, F3 Suchen, F4 Sortieren ({ $sort }), INS Neu, DEL Löschen
icbsm_user_list_search=Suche: { $search }_ (ENTER Behalten, ESC Leeren)
icbsm_user_list_filtered=Suche „{ $search }“: { $count } angezeigt, sortiert nach { $sort } – F3 Suchen, F4 Sortieren

user_editor_name=Name
user_editor_name-status=Name
user_editor_name-help=
    # Name

    Name für die Anmeldung. Eine Änderung betrifft auch die künftige
    Anmeldung; der Name muss deshalb in der Benutzerdatei eindeutig bleiben.
user_editor_alias=Alias
user_editor_alias-status=Alias
user_editor_alias-help=
    # Alias

    Pseudonym für Konferenzen, die Aliasse erlauben.
user_editor_password=Passwort
user_editor_password-status=Passwort
user_editor_password-help=
    # Passwort

    Anmeldepasswort des Benutzers. Es wird nach der eingestellten Methode
    gespeichert und kann bei aktivierter Hashspeicherung hier nicht ausgelesen werden.
user_editor_security=Sicherheitsstufe
user_editor_security-status=Sicherheitsstufe
user_editor_security-help=
    # Sicherheitsstufe

    Bestimmt den Zugriff auf Befehle, Konferenzen und Dateien.
    0 sperrt das Konto.
user_editor_city=Ort
user_editor_city-status=Ort
user_editor_city-help=
    # Ort

    Vom Benutzer angegebener Herkunftsort. Er erscheint in WHO,
    wenn das Board zur Ortsanzeige eingerichtet ist.
user_editor_bus_phone=Gesch./Daten-Tel.
user_editor_bus_phone-status=Geschäfts-/Datentelefon
user_editor_bus_phone-help=
    # Geschäfts-/Datentelefon

    Geschäfts- oder Datentelefonnummer des Benutzers.
user_editor_home_phone=Privat/Sprach-Tel.
user_editor_home_phone-status=Privat-/Sprachtelefon
user_editor_home_phone-help=
    # Privat-/Sprachtelefon

    Private oder Sprachtelefonnummer des Benutzers.
user_editor_verify_answer=Prüfantwort
user_editor_verify_answer-status=Antwort zur Identitätsprüfung
user_editor_verify_answer-help=
    # Prüfantwort

    Antwort auf die Identitätsfrage. Bei der Wiederherstellung eines
    Kontos danach fragen, um die Identität zu bestätigen.
user_editor_protocol=Protokoll
user_editor_protocol-status=Protokoll
user_editor_protocol-help=
    # Protokoll

    Übertragungsprotokoll für Uploads und Downloads dieses Benutzers.
user_editor_page_len=Seitenlänge
user_editor_page_len-status=Seitenlänge
user_editor_page_len-help=
    # Seitenlänge

    Zeilenanzahl vor einer Ausgabepause für diesen Benutzer.
    0 schaltet die Pause aus und lässt alles durchlaufen.
user_editor_reg_ex_date=Ablaufdatum
user_editor_reg_ex_date-status=Ablaufdatum der Registrierung
user_editor_reg_ex_date-help=
    # Ablaufdatum

    Tag, an dem das Abonnement endet. Wird nur bei aktiviertem
    Abonnementmodus geprüft; ein leeres Datum läuft nie ab.
user_editor_exp_sec=Ablaufstufe
user_editor_exp_sec-status=Sicherheitsstufe nach Ablauf
user_editor_exp_sec-help=
    # Ablaufstufe

    Auf diese Sicherheitsstufe fällt der Benutzer nach dem Ablaufdatum zurück.
user_editor_msg_clear=Bildschirm löschen
user_editor_msg_clear-status=Bildschirm zwischen Nachrichten löschen
user_editor_msg_clear-help=
    # Bildschirm löschen

    Legt fest, ob der Bildschirm zwischen Nachrichten gelöscht wird.
user_editor_scroll_msg=Nachrichten rollen
user_editor_scroll_msg-status=Nachrichtentext ohne Seitenpausen ausgeben
user_editor_scroll_msg-help=
    # Nachrichten rollen

    Legt fest, ob Nachrichtentexte durchlaufen statt seitenweise anzuhalten.
user_editor_fse_mode=Vollbildeditor
user_editor_fse_mode-status=Vollbildeditor
user_editor_fse_mode-help=
    # Vollbildeditor

    Legt fest, ob Nachrichten mit dem Vollbildeditor statt zeilenweise geschrieben werden.
user_editor_use_short_filedescr=Kurzbeschreibung
user_editor_use_short_filedescr-status=Kurze Dateibeschreibungen
user_editor_use_short_filedescr-help=
    # Kurzbeschreibung

    Legt fest, ob Dateilisten nur die erste Zeile jeder Beschreibung zeigen.
user_editor_wide_editor=79-Spalten-Editor
user_editor_wide_editor-status=79-Spalten-Editor
user_editor_wide_editor-help=
    # 79-Spalten-Editor

    Erlaubt das Schreiben über volle 79 Spalten statt in der schmaleren Standardbreite.
user_editor_last_conference=Zuletzt in
user_editor_last_conference-status=Zuletzt besuchte Konferenz
user_editor_last_conference-help=
    # Letzte Konferenz

    Konferenz bei der letzten Abmeldung. Ein automatischer
    Wiederbeitritt führt den Benutzer dorthin zurück.
user_editor_long_msg_header=Lange Köpfe
user_editor_long_msg_header-status=Vollständige Nachrichtenköpfe
user_editor_long_msg_header-help=
    # Lange Köpfe

    Legt fest, ob vollständige Nachrichtenköpfe mit allen Zeilen
    oder die Kurzform angezeigt werden.
user_editor_delete_user=Benutzer löschen
user_editor_delete_user-status=Benutzer löschen
user_editor_delete_user-help=
    # Benutzer löschen

    Markiert den Datensatz zum Löschen. Das Konto bleibt bis zum Packen
    der Benutzerdatei erhalten; bis dahin lässt sich die Markierung zurücknehmen.
user_editor_chat_status=Chatstatus
user_editor_chat_status-status=Chatstatus
user_editor_chat_status-help=
    # Chatstatus

    Legt fest, ob der Benutzer für Chats mit anderen Nodes verfügbar ist
    oder ungestört bleiben möchte.
user_editor_expert_mode=Experte
user_editor_expert_mode-status=Expertenmodus
user_editor_expert_mode-help=
    # Expertenmodus

    Verwendet kurze Eingabeaufforderungen statt vollständiger Menüs.
user_editor_comment1=Kommentar 1
user_editor_comment1-status=Benutzerkommentar
user_editor_comment1-help=
    # Kommentar 1

    Zeile, die der Benutzer bei der Registrierung über sich selbst geschrieben hat.
user_editor_comment2=Kommentar 2
user_editor_comment2-status=Sysop-Kommentar
user_editor_comment2-help=
    # Kommentar 2

    Nur für den Sysop sichtbare Notiz, etwa zum Grund einer Hochstufung
    oder dafür, dass ein Konto beobachtet wird.
user_editor_adr1=Adresse #1
user_editor_adr1-status=Adresse #1
user_editor_adr1-help=
    # Adresse #1

    Erste Zeile der Postanschrift des Benutzers.
user_editor_adr2=Adresse #2
user_editor_adr2-status=Adresse #2
user_editor_adr2-help=
    # Adresse #2

    Zweite Zeile der Postanschrift des Benutzers.
user_editor_state=Region
user_editor_state-status=Region
user_editor_state-help=
    # Region

    Bundesland oder Region der Postanschrift des Benutzers.
user_editor_zip=Postleitzahl
user_editor_zip-status=Postleitzahl
user_editor_zip-help=
    # Postleitzahl

    Postleitzahl der Benutzeradresse.
user_editor_country=Land
user_editor_country-status=Land
user_editor_country-help=
    # Land

    Land der Benutzeradresse.
user_editor_cmt_line1=Zeile 1
user_editor_cmt_line1-status=Freie Kommentarzeile 1
user_editor_cmt_line1-help=
    # Freie Kommentarzeile 1

    Frei verwendbare Zeile über diesen Benutzer. Sie erscheint dort,
    wo das Board die freien Kommentarzeilen ausgibt.
user_editor_cmt_line2=Zeile 2
user_editor_cmt_line2-status=Freie Kommentarzeile 2
user_editor_cmt_line2-help=
    # Freie Kommentarzeile 2

    Weitere frei verwendbare Zeile über diesen Benutzer.
user_editor_cmt_line3=Zeile 3
user_editor_cmt_line3-status=Freie Kommentarzeile 3
user_editor_cmt_line3-help=
    # Freie Kommentarzeile 3

    Weitere frei verwendbare Zeile über diesen Benutzer.
user_editor_cmt_line4=Zeile 4
user_editor_cmt_line4-status=Freie Kommentarzeile 4
user_editor_cmt_line4-help=
    # Freie Kommentarzeile 4

    Weitere frei verwendbare Zeile über diesen Benutzer.
user_editor_cmt_line5=Zeile 5
user_editor_cmt_line5-status=Freie Kommentarzeile 5
user_editor_cmt_line5-help=
    # Freie Kommentarzeile 5

    Letzte frei verwendbare Zeile über diesen Benutzer.
user_editor_gender=Geschlecht
user_editor_gender-status=Vom Benutzer angegebenes Geschlecht
user_editor_gender-help=
    # Geschlecht

    Geschlechtsangabe des Benutzers, sofern das Board danach fragt.
user_editor_birthdate=Geburtsdatum
user_editor_birthdate-status=Geburtsdatum
user_editor_birthdate-help=
    # Geburtsdatum

    Geburtsdatum des Benutzers, das PPE-Programme für Altersprüfungen lesen.
user_editor_email=E-Mail-Adresse
user_editor_email-status=E-Mail
user_editor_email-help=
    # E-Mail-Adresse

    E-Mail-Adresse des Benutzers.
user_editor_web=Webadresse
user_editor_web-status=Web
user_editor_web-help=
    # Webadresse

    Adresse der Homepage des Benutzers.

# ICBSetup -> Konferenzen -> DIRS.TOML bearbeiten
dirs_editor_title=DIR.LST-Editor { $conference }
dirs_table_name_header=Name
dirs_table_path_header=Pfad
dirs_edit_directory_title=Verzeichnis bearbeiten
dirs_edit_name=Name
dirs_edit_name-status=Name
dirs_edit_name-help=
    # Name

    Name des Dateiverzeichnisses, wie Benutzer ihn im Dateimenü sehen.
dirs_edit_path=Pfad
dirs_edit_path-status=Pfad
dirs_edit_path-help=
    # Pfad

    Verzeichnis auf dem Datenträger mit den hier angebotenen Dateien.
dirs_metadata_path=Metadatenpfad
dirs_metadata_path-status=Speichert zusätzliche Dateiinformationen
dirs_metadata_path-help=
    # Metadatenpfad

    Speicherort für Angaben über die Dateien hinaus:
    Beschreibungen, Uploader und Downloadzähler.
dirs_edit_password=Passwort
dirs_edit_password-status=Passwort
dirs_edit_password-help=
    # Passwort

    Vor dem Öffnen des Verzeichnisses benötigtes Passwort.
    Schützt den Zugriff, ohne eine höhere Sicherheitsstufe zu vergeben.
dirs_edit_fido_tag=Fido-Bereichskennung
dirs_edit_fido_tag-status=Dateiecho-Kennung dieses Verzeichnisses (LEER=lokales Verzeichnis)
dirs_edit_fido_tag-help=
    # Fido-Bereichskennung

    Name des Dateiverzeichnisses im Fido-Technologienetz, etwa R24NODEL.
    Eine Datei mit dieser Kennung in ihrer TIC-Datei wird hier abgelegt.
    Für Verzeichnisse ohne Dateiecho leer lassen. Ein Verzeichnis,
    dessen Name selbst der Kennung entspricht, wird unter diesem Namen gefunden.
dirs_edit_sort=Sortierung
dirs_edit_sort-status=Sortierung
dirs_edit_sort-help=
    # Sortierung

    Sortierschlüssel der Liste: Dateiname oder Datum.
dirs_edit_sort_asc=Aufsteigend sortieren
dirs_edit_sort_asc-status=Aufsteigend sortieren
dirs_edit_sort_asc-help=
    # Aufsteigend sortieren

    Auf- oder absteigende Reihenfolge. Absteigende Datumssortierung
    zeigt die neuesten Dateien zuerst.
dirs_edit_has_new_files=Neue Dateien suchen
dirs_edit_has_new_files-status=Bei der Suche nach neuen Dateien berücksichtigen
dirs_edit_has_new_files-help=
    # Neue Dateien suchen

    Legt fest, ob die Suche nach neuen Dateien dieses Verzeichnis einbezieht.
    Für unveränderliche Inhalte ausschalten.
dirs_edit_is_free=Kostenlos
dirs_edit_is_free-status=Kostenlos
dirs_edit_is_free-help=
    # Kostenlos

    Downloads hieraus zählen weder gegen das Bytelimit noch gegen das
    Verhältnis des Benutzers. Geeignet für eigene Hilfsprogramme und Dokumentation.
dirs_edit_list_sec=Anzeigerechte
dirs_edit_list_sec-status=Benötigte Rechte zum Anzeigen
dirs_edit_list_sec-help=
    # Anzeigerechte

    Erforderliche Rechte, um dieses Verzeichnis überhaupt zu sehen.
    Ohne diese Rechte bleibt selbst seine Existenz verborgen.
dirs_download_sec=Downloadrechte
dirs_download_sec-status=Benötigte Rechte für Downloads
dirs_download_sec-help=
    # Downloadrechte

    Erforderliche Rechte, um Dateien aus diesem Verzeichnis herunterzuladen.
    Sie können höher sein als die Rechte zum Anzeigen der Liste.
area_editor_title=AREA.LST-Editor – { $conference }
area_editor_edit_title=Nachrichtenbereich bearbeiten
area_editor_key_help=↑ Auf  ↓ Ab  INS Neu  F2 Import  ⌫ Löschen  PgUp/Dn Verschieben  ␛ Zurück
area_import_title=Bereichsliste importieren
area_import_file=Bereichslistendatei
area_import_file-status=FIDONET.NA oder eine .NA-Bereichsliste eines anderen Netzes
area_import_file-help=
    # Bereichslistendatei

    Textdatei, deren Nicht-Kommentarzeilen mit einer Echomail-Kennung
    beginnen, gefolgt vom sichtbaren Bereichsnamen. FIDONET.NA und
    netzspezifische .NA-Dateien verwenden dieses Format.
area_import_directory=Basisverzeichnis
area_import_directory-status=Speicherort der importierten JAM-Nachrichtenbasen
area_import_directory-help=
    # Basisverzeichnis

    Jede ausgewählte Kennung erhält unter diesem Verzeichnis einen
    sicheren, kleingeschriebenen Pfad. Vorhandene Pfade und Kennungen
    werden niemals überschrieben.
area_import_load_help=F2 Laden  ␛ Zurück
area_import_preview_title=Zu importierende Bereiche wählen
area_import_preview_help=↑/↓ Bewegen  Space Auswählen  Enter Importieren  ␛ Zurück
area_import_empty=Die Datei enthält keine gültigen Bereichseinträge.
area_import_failed=Bereichsliste konnte nicht gelesen werden: { $error }
area_import_done={ $count } Nachrichtenbereiche importiert. Zum Behalten den AREA.LST-Editor speichern.
area_editor_name=Name
area_editor_name-status=Name
area_editor_name-help=
    # Name

    Name des Nachrichtenbereichs, wie Benutzer ihn sehen.
area_editor_qwk_name=QWK-Name
area_editor_qwk_name-status=QWK-Name (LEER=Name verwenden)
area_editor_qwk_name-help=
    Bereichsname, wie er in QWK-Paketen erscheint.
area_editor_fido_tag=Fido-Bereichskennung
area_editor_fido_tag-status=Echomail-Kennung dieses Bereichs (LEER=lokaler Bereich)
area_editor_fido_tag-help=
    # Fido-Bereichskennung

    Name des Bereichs im Fido-Technologienetz, etwa FSX_GEN.
    Für einen rein lokalen Bereich leer lassen.
area_editor_fido_origin=Fido-Origin
area_editor_fido_origin-status=Origin dieses Bereichs (LEER=boardweite Origin)
area_editor_fido_origin-help=
    # Fido-Origin

    Origin-Zeile für hier geschriebene Echomail. Leer lassen, um die
    boardweite Origin aus der Fido-Konfiguration zu verwenden.
area_editor_file=Datei
area_editor_file-status=Datei
area_editor_file-help=
    # Datei

    Nachrichtenbasis auf dem Datenträger für diesen Bereich.
area_editor_is_readonly=Schreibgeschützt
area_editor_is_readonly-status=Schreibgeschützt
area_editor_is_readonly-help=
    # Schreibgeschützt

    Benutzer dürfen diesen Bereich lesen, aber nicht darin schreiben.
area_editor_allow_aliases=Aliasse erlauben
area_editor_allow_aliases-status=Aliasse erlauben
area_editor_allow_aliases-help=
    # Aliasse erlauben

    Erlaubt Nachrichten unter einem Alias statt unter dem echten Namen des Benutzers.
area_editor_list_sec=Leserechte
area_editor_list_sec-status=Benötigte Rechte zum Anzeigen und Lesen
area_editor_list_sec-help=
    # Leserechte

    Erforderliche Rechte, um diesen Bereich zu sehen und zu lesen.
area_editor_enter_sec=Schreibrechte
area_editor_enter_sec-status=Benötigte Rechte zum Schreiben
area_editor_enter_sec-help=
    # Schreibrechte

    Erforderliche Rechte zum Schreiben einer Nachricht in diesem Bereich.
area_editor_attach_sec=Anhangrechte
area_editor_attach_sec-status=Benötigte Rechte für Anhänge
area_editor_attach_sec-help=
    # Anhangrechte

    Erforderliche Rechte, um hier eine Datei an eine Nachricht anzuhängen.
area_editor_qwk_number=QWK-Nummer
area_editor_qwk_number-status=QWK-Nummer (0=automatisch)
area_editor_qwk_number-help=
    Bereichsnummer in QWK-Paketen. Erlaubt feste Nummern in den Paketen.
    So lassen sich Bereiche hinzufügen oder entfernen, ohne die QWK-Nummern zu verändern.

doors_editor_title=DOORS-Dateieditor { $conference }
doors_editor_edit_title=Door bearbeiten
doors_editor_key_help=↑ Auf  ↓ Ab  F2 Neues Door  Tab Doors bearbeiten  ␛ Zurück
doors_editor_key_help_door=↑ Auf  ↓ Ab  F2/INS Neu  ␡ Löschen  Tab BBSLINK bearbeiten  ␛ Zurück
doors_editor_header_door=Door
doors_editor_header_description=Beschreibung
doors_editor_header_type=Typ
doors_editor_bbslink_credentials=BBSLink-Zugangsdaten
doors_editor_system_code=Systemcode
doors_editor_auth_code=Authentifizierungscode
doors_editor_scheme_code=Schemacode
door_editor_dos_command=DOS-Befehl
door_editor_dos_memory=DOS-Speicher (MB)
door_editor_dos_max_seconds=DOS-Zeitlimit (Sek.)
door_editor_name=Name
door_editor_name-status=Name
door_editor_name-help=
    # Name

    Name, den Benutzer zum Starten dieses Doors eingeben.
door_editor_description=Beschreibung
door_editor_description-status=Beschreibung
door_editor_description-help=
    # Beschreibung

    Beschreibungszeile für dieses Door in der Door-Liste.
door_editor_password=Passwort
door_editor_password-status=Passwort
door_editor_password-help=
    # Passwort

    Vor dem Start des Doors erforderliches Passwort.
door_editor_path=Pfad
door_editor_path-status=Pfad
door_editor_path-help=
    # Pfad

    Programm, das für dieses Door ausgeführt wird.
door_editor_door_type=Door-Typ
door_editor_door_type-status=Door-Typ
door_editor_door_type-help=
    # Door-Typ

    Art des Programms und Übergabe der Benutzersitzung an das Door.
door_editor_security=Zugriffsrechte
door_editor_security-status=Erforderlicher Sicherheitsausdruck
door_editor_security-help=
    # Zugriffsrechte

    Sicherheitsausdruck, den ein Benutzer vor dem Door-Start erfüllen muss.
door_editor_drop_file=Drop-Datei
door_editor_drop_file-status=Vor dem Door-Start erzeugte Drop-Datei
door_editor_drop_file-help=
    # Drop-Datei

    BBS-Sitzungsdatei, die vor dem Start im Door-Verzeichnis erzeugt wird.
    DOS-Doors erhalten dieselbe Datei in C:\DOOR und C:\ICB.
door_editor_use_shell_execute=Über Shell ausführen
door_editor_use_shell_execute-status=Über die System-Shell ausführen
door_editor_use_shell_execute-help=
    # Über Shell ausführen

    Startet das Door über die System-Shell. Befehlszeilen mit Argumenten
    oder Umleitungen werden so wie bei manueller Eingabe interpretiert.

lang_editor_title=Sprachtabelle
lang_editor_header_language=Sprache
lang_editor_header_ext=Erweiterung
lang_editor_header_locale=Gebietsschema
lang_editor_header_yes=Ja
lang_editor_header_no=Nein
lang_editor_edit_lang=Sprache bearbeiten
lang_editor_edit_lang_label=Sprache
lang_editor_edit_lang_label-status=Sprache
lang_editor_edit_lang_label-help=
    # Sprache

    Sprachname, wie er bei der Anmeldung zur Auswahl angeboten wird.
lang_editor_edit_extension=Erweiterung
lang_editor_edit_extension-status=Erweiterung
lang_editor_edit_extension-help=
    # Erweiterung

    Erweiterung der Anzeige- und Textdateien dieser Sprache.
    Damit findet das Board die richtige Sprachversion eines Bildschirms.
lang_editor_edit_locale=Gebietsschema
lang_editor_edit_locale-status=Gebietsschema
lang_editor_edit_locale-help=
    # Gebietsschema

    Gebietsschema dieser Sprache. Es bestimmt die Darstellung von
    Datumsangaben und Zahlen für Benutzer.
lang_editor_edit_yes_char=Ja-Taste
lang_editor_edit_yes_char-status=Ja-Taste
lang_editor_edit_yes_char-help=
    # Ja-Taste

    Taste für Ja in dieser Sprache. So können Benutzer in ihrer
    eigenen Sprache statt mit Y antworten.
lang_editor_edit_no_char=Nein-Taste
lang_editor_edit_no_char-status=Nein-Taste
lang_editor_edit_no_char-help=
    # Nein-Taste

    Taste für Nein in dieser Sprache.
surveys_editor_title=Umfragen { $conference }
survey_editor_editor=Umfrage bearbeiten
survey_editor_editor_header_question=Frage
survey_editor_editor_header_answer=Antwort
survey_editor_editor_file=Umfragedatei
survey_editor_editor_file-status=Umfragedatei
survey_editor_editor_file-help=
    # Umfragedatei

    Datei mit den Fragen dieser Umfrage, eine pro Zeile.
survey_editor_editor_answer_file=Antwortdatei
survey_editor_editor_answer_file-status=Antwortdatei
survey_editor_editor_answer_file-help=
    # Antwortdatei

    Hier werden die Antworten gesammelt. Die Antworten jedes Benutzers
    werden angehängt; die Datei wächst mit jeder Teilnahme.
survey_editor_editor_security=Zugriffsrechte
survey_editor_editor_security-status=Zugriffsrechte
survey_editor_editor_security-help=
    # Zugriffsrechte

    Benötigte Rechte, damit einem Benutzer diese Umfrage angeboten wird.

sec_level_editor_title=Sicherheitsstufen bearbeiten
sec_level_editor_editor=Sicherheitsstufe bearbeiten
sec_level_header_security=Stufe
sec_level_header_description=Beschreibung
sec_level_header_time=Zeit
sec_level_editor_security=Sicherheitsstufe
sec_level_editor_security-status=Sicherheitsstufe
sec_level_editor_security-help=
    # Sicherheitsstufe

    Sicherheitsstufe dieses Eintrags. Alle folgenden Angaben gelten
    für Benutzer dieser Stufe.
sec_level_editor_description=Beschreibung
sec_level_editor_description-status=Beschreibung
sec_level_editor_description-help=
    # Beschreibung

    Notiz zum Zweck dieser Stufe. Nur als Hilfe für den Sysop gedacht.
sec_level_editor_password=Passwort
sec_level_editor_password-status=Passwort
sec_level_editor_password-help=
    # Passwort

    Die Eingabe dieses Passworts hebt einen Benutzer auf diese Stufe an,
    ohne dass der Sysop den Datensatz bearbeiten muss.
sec_level_editor_time_per_day=Zeit
sec_level_editor_time_per_day-status=Zeit
sec_level_editor_time_per_day-help=
    # Zeit

    Erlaubte Onlineminuten pro Tag für Benutzer dieser Stufe.
sec_level_editor_daily_bytes=Tägliche KB
sec_level_editor_daily_bytes-status=Tägliches Downloadkontingent in KB
sec_level_editor_daily_bytes-help=
    # Tägliche KB

    Erlaubte Downloadmenge in Kilobytes pro Tag für Benutzer dieser Stufe.
sec_level_editor_file_ratio=Dateiverhältnis
sec_level_editor_file_ratio-status=Dateiverhältnis
sec_level_editor_file_ratio-help=
    # Dateiverhältnis

    Anzahl erlaubter Downloads pro hochgeladener Datei. Greift erst,
    wenn das unten angegebene freie Dateikontingent aufgebraucht ist.
sec_level_editor_byte_ratio=Byteverhältnis
sec_level_editor_byte_ratio-status=Byteverhältnis
sec_level_editor_byte_ratio-help=
    # Byteverhältnis

    Erlaubte Downloadbytes je Uploadbyte, nach demselben Prinzip wie beim Dateiverhältnis.
sec_level_editor_file_limit=Freie Dateien
sec_level_editor_file_limit-status=Dateilimit vor Anwendung des Verhältnisses
sec_level_editor_file_limit-help=
    # Freie Dateien

    Anzahl erlaubter Downloads, bevor das Dateiverhältnis gilt.
    Damit können neue Benutzer etwas herunterladen, bevor sie selbst Dateien hochladen.
sec_level_editor_kb_limit=Freie KB
sec_level_editor_kb_limit-status=Kilobytelimit vor Anwendung des Verhältnisses
sec_level_editor_kb_limit-help=
    # Freie KB

    Erlaubte Downloadmenge in Kilobytes, bevor das Byteverhältnis gilt.
sec_level_editor_file_credit=Dateiguthaben
sec_level_editor_file_credit-status=Dateiguthaben
sec_level_editor_file_credit-help=
    # Dateiguthaben

    Zusätzlich zum Verhältnis gewährte Dateien als Startguthaben für Benutzer dieser Stufe.
sec_level_editor_kb_credit=KB-Guthaben
sec_level_editor_kb_credit-status=Kilobyteguthaben
sec_level_editor_kb_credit-help=
    # KB-Guthaben

    Zusätzlich zum Byteverhältnis gewährte Kilobytes.
sec_level_editor_enforce_time=Zeitlimit durchsetzen
sec_level_editor_enforce_time-status=Zeitlimit durchsetzen
sec_level_editor_enforce_time-help=
    # Zeitlimit durchsetzen

    Legt fest, ob das tägliche Zeitlimit für diese Stufe gilt.
    Zusammen mit der gleichnamigen Boardoption bestimmt dies,
    ob die Zeit pro Tag gezählt wird.
sec_level_editor_allow_alias=Alias erlauben
sec_level_editor_allow_alias-status=Alias erlauben
sec_level_editor_allow_alias-help=
    # Alias erlauben

    Erlaubt Benutzern dieser Stufe, unter einem Alias aufzutreten.
sec_level_force_read_mail=Nachrichtenlesen erzwingen
sec_level_force_read_mail-status=Lesen wartender Nachrichten erzwingen
sec_level_force_read_mail-help=
    # Nachrichtenlesen erzwingen

    Benutzer dieser Stufe müssen zuerst ihre wartenden Nachrichten lesen,
    bevor sie etwas anderes tun dürfen. So sind wichtige Hinweise nicht zu umgehen.
sec_level_demo_acc=Demokonto
sec_level_demo_acc-status=Demokonto
sec_level_demo_acc-help=
    # Demokonto

    Kennzeichnet diese Stufe als Konto zum Umsehen. Besucher können
    das Board ansehen, ohne dass dies als echte Registrierung zählt.
sec_level_enable_acc=Abrechnungskonto aktivieren
sec_level_enable_acc-status=Abrechnungskonto aktivieren
sec_level_enable_acc-help=
    # Abrechnungskonto aktivieren

    Legt fest, ob Benutzer dieser Stufe an der Abrechnung teilnehmen.
    Ohne diese Option wird ihr Guthaben weder belastet noch geprüft.

protocol_editor_title=Übertragungsprotokolle
protocol_editor_editor=Protokoll bearbeiten
protocol_editor_header_char_code=Taste
protocol_editor_header_description=Beschreibung
protocol_editor_is_enabled=Aktiviert
protocol_editor_is_enabled-status=Aktiviert
protocol_editor_is_enabled-help=
    # Aktiviert

    Legt fest, ob das Protokoll angeboten wird. Ausschalten blendet
    es aus der Protokollliste aus, ohne den Eintrag zu löschen.
protocol_editor_is_batch=Stapel
protocol_editor_is_batch-status=Stapelübertragung
protocol_editor_is_batch-help=
    # Stapelübertragung

    Legt fest, ob das Protokoll mehrere Dateien pro Übertragung unterstützt.
    Damit lassen sich mehrere Dateien markieren und gemeinsam übertragen.
protocol_editor_bidirectional=Bidirektional
protocol_editor_bidirectional-status=Bidirektional
protocol_editor_bidirectional-help=
    # Bidirektional

    Legt fest, ob das Protokoll gleichzeitig senden und empfangen kann.
protocol_editor_char_code=Taste
protocol_editor_char_code-status=Auswahltaste
protocol_editor_char_code-help=
    # Auswahltaste

    Taste zur Auswahl dieses Protokolls.
protocol_editor_description=Beschreibung
protocol_editor_description-status=Beschreibung
protocol_editor_description-help=
    # Beschreibung

    Beschreibung des Protokolls in der Auswahlliste für Benutzer.
protocol_editor_send_cmd=Sendebefehl
protocol_editor_send_cmd-status=Sendebefehl
protocol_editor_send_cmd-help=
    # Sendebefehl

    Befehl zum Senden von Dateien mit einem externen Protokoll.
    Für ein intern vom Board unterstütztes Protokoll leer lassen.
protocol_editor_recv_cmd=Empfangsbefehl
protocol_editor_recv_cmd-status=Empfangsbefehl
protocol_editor_recv_cmd-help=
    # Empfangsbefehl

    Befehl zum Empfangen von Dateien mit einem externen Protokoll.
command_editor_title=CMD.LST-Editor
command_editor_header_command=Befehl
command_editor_header_action=Aktion
command_editor_header_parameter=Parameter
command_editor_editor=Befehl bearbeiten
command_editor_keyword=Schlüsselwort
command_editor_help=Hilfe
command_editor_security=Zugriffsrechte
command_editor_action=Aktion
command_editor_parameter=Parameter
command_editor_command_type=Befehlstyp

msg_networking_title=Nachrichtennetze
msg_networking_qwk=QWK-Einstellungen
msg_networking_ftn=Fido-Konfiguration
fido_config_title=Fido-Konfiguration
fido_menu_processing=Fido-Konfiguration
fido_menu_tosser=Tosser-Konfiguration
fido_menu_nodes=Node-Konfiguration
fido_menu_addresses=Systemadresse
fido_menu_directories=Dateien und Verzeichnisse
fido_menu_routing=Routing-Konfiguration
fido_menu_freq_paths=FREQ-Pfadliste
fido_menu_freq_restrictions=FREQ-Beschränkungen
fido_menu_freq_magic=FREQ-Sammelnamen
fido_menu_freq_deny=FREQ-Node-Sperrliste
fido_freq_title=Fido-FREQ-Beschränkungen
fido_freq_enabled=Dateianforderungen beantworten
fido_freq_enabled-status=Nodes dürfen Dateien bei diesem Board anfordern
fido_freq_enabled-help=
    # Dateianforderungen beantworten

    Eine Dateianforderung nennt Dateien, statt sie über das Board
    herunterzuladen. Ausgeschaltet bleiben Anforderungen liegen,
    und es wird nichts gesendet.
fido_freq_session_kbytes=Maximale Sitzungs-KB
fido_freq_session_kbytes-status=Maximale Datenmenge je Anforderung in Kilobytes (0=unbegrenzt)
fido_freq_session_kbytes-help=
    # Maximale Sitzungs-KB

    Eine Anforderung stoppt, sobald die angeforderten Dateien diese Größe
    erreichen. PCBoard nannte das Feld Bytes, zählte aber Kilobytes.
    Hier gilt dasselbe, damit der konfigurierte Wert nach einem Import seine Bedeutung behält.
fido_freq_daily_kbytes=Maximale Tages-KB
fido_freq_daily_kbytes-status=Maximale Datenmenge je Node und Tag in Kilobytes (0=unbegrenzt)
fido_freq_daily_kbytes-help=
    # Maximale Tages-KB

    Alle Anforderungen eines Nodes zählen bis zum Datumswechsel gegen
    dieses Limit. Wie der Sitzungswert wird es in Kilobytes angegeben.
fido_freq_path_title=FREQ-Pfadkonfiguration
fido_freq_path_editor=FREQ-Pfad
fido_freq_header_path=Pfad
fido_freq_header_password=Passwort
fido_freq_header_file=Datei
fido_freq_header_magic=Sammelname
fido_freq_header_node=Node
fido_freq_path=Pfad
fido_freq_path-status=Verzeichnis für die Beantwortung von Dateianforderungen
fido_freq_path-help=
    # Pfad

    Nur direkt in diesen Verzeichnissen liegende Dateien können angefordert
    werden. Anforderungen anderer Dateien oder Versuche, diese Verzeichnisse
    zu verlassen, werden abgelehnt.
fido_freq_password=Passwort
fido_freq_password-status=Benötigtes Passwort für diesen Zugriff (LEER=für alle offen)
fido_freq_password-help=
    # Passwort

    Wenn gesetzt, werden nur Anforderungen mit demselben Passwort
    aus diesem Verzeichnis beantwortet. Der Node gibt es nach dem Dateinamen an.
fido_freq_magic_title=Fidonet-Sammelnamen bearbeiten
fido_freq_magic_editor=FREQ-Sammelname
fido_freq_magic=Sammelname
fido_freq_magic-status=Name, den ein Node anfordert
fido_freq_magic-help=
    # Sammelname

    Ein Name, der stellvertretend für eine gewählte Datei steht.
    Die anfordernde Seite muss deren tatsächlichen Namen nicht kennen.
    FILES und NODEDIFF sind die von Fidonet erwarteten Namen.
fido_freq_file=Datei
fido_freq_file-status=Datei, die der Sammelname ausliefert
fido_freq_deny_title=FREQ-Node-Sperrliste
fido_freq_deny_editor=Gesperrter Node
fido_freq_node=Node
fido_freq_node-status=Node, dessen Dateianforderungen abgelehnt werden
fido_freq_node-help=
    # Node

    Adresse eines Nodes, dessen Anforderungen immer abgelehnt werden,
    unabhängig von angeforderter Datei und mitgesendetem Passwort.
fido_processing_title=Fido-Verarbeitung
fido_enabled=Fido-Verarbeitung aktivieren
fido_enabled-status=Board am Fido-Netz teilnehmen lassen
fido_enabled-help=
    # Fido-Verarbeitung aktivieren

    Schaltet Abrufe, Import und Export ein oder aus, ohne Systemadressen,
    Node-Konfiguration oder Pfade zu verlieren.
fido_import_after_xfer=Direkt nach Übertragung importieren
fido_import_after_xfer-status=Eingang nach Verbindungsende verarbeiten
fido_import_after_xfer-help=Empfangene Daten werden sofort in die Nachrichtenbasen eingelesen, statt auf den nächsten Tosser-Lauf zu warten.
fido_process_in=Eingehende Pakete verarbeiten
fido_process_in-status=Wartende Nachrichten aus dem Eingangsverzeichnis lesen
fido_process_in-help=Ausgeschaltet wird nichts aus dem Eingangsverzeichnis eingelesen. So lässt sich das Board aus dem Netz nehmen, ohne ankommende Daten zu verlieren.
fido_process_out=Nachrichtenexport erlauben
fido_process_out-status=Lokal geschriebene Nachrichten für Gegenstellen verpacken
fido_process_out-help=Ausgeschaltet verlassen keine hier geschriebenen Nachrichten das Board.
fido_dial_out=Ausgehende Verbindungen erlauben
fido_dial_out-status=Board darf Gegenstellen anrufen
fido_dial_out-help=Ein Board, das ausschließlich angerufen wird, lässt diese Option aus.
fido_process_orphan=Fremdadressierte Pakete verarbeiten
fido_process_orphan-status=An andere Systeme adressierte Pakete lesen
fido_process_orphan-help=Ein Hub liest auch Post an andere Systeme; ein End-Node sollte das nicht tun. Ausgeschaltet bleiben fremdadressierte Pakete im Eingangsverzeichnis.
fido_log_level=Fido-Protokollumfang
fido_log_level-status=Umfang der Meldungen über den Mailer-Betrieb
fido_log_level-help=
    # Fido-Protokollumfang

    Normal meldet Warnungen und Fehler. Detailliert erfasst zusätzlich
    reguläre Mailer-Aktivitäten. Debug enthält Protokolldetails und hilft
    bei der Diagnose neuer oder fehlerhafter Nodes.
fido_log_level_normal=Normal
fido_log_level_detailed=Detailliert
fido_log_level_debug=Debug
fido_default_zone=Standardzone
fido_default_zone-status=Zone zur Ergänzung zweidimensionaler Pakete
fido_default_zone-help=Alte Pakete lassen die Zone auf null; nur der Sysop weiß dann, welches Netz gemeint war.
fido_default_net=Standardnetz
fido_default_net-status=Netz zur Ergänzung zweidimensionaler Pakete
fido_default_net-help=
    # Standardnetz

    Netznummer zur Ergänzung eingehender Adressen ohne Netzangabe,
    wie sie bei älteren zweidimensionalen Paketen vorkommen.
fido_tosser_title=Fido-Tosser-Konfiguration
fido_enable_routing=Eingehendes Routing aktivieren
fido_enable_routing-status=An andere Systeme adressierte Pakete weiterleiten
fido_enable_routing-help=Nutzt die Routing-Konfiguration, um ein Paket im Ausgang des nächsten Nodes abzulegen, statt es hier zu importieren.
fido_check_dupe_path=Duplikate anhand des Pfads prüfen
fido_check_dupe_path-status=Nachricht verwerfen, deren Pfad dieses Board bereits enthält
fido_check_dupe_path-help=Eine Nachricht, die bereits hier war, ist auf einem Umweg zurückgekehrt. Ohne Nachrichtenkennung erkennt die ID-Prüfung diesen Fall nicht.
fido_check_dupe_msg_id=Duplikate anhand MSGID prüfen
fido_check_dupe_msg_id-status=Nachricht mit bereits bekannter Kennung im Bereich verwerfen
fido_check_dupe_msg_id-help=Nachrichten können über mehrere Wege durch ein Netz laufen und mehrfach eintreffen. Anhand der Nachrichtenkennung werden diese Kopien erkannt.
fido_msgs_to_track=Nachrichten für Duplikatprüfung
fido_msgs_to_track-status=Prüftiefe der Duplikatsuche; 0 prüft den ganzen Bereich
fido_msgs_to_track-help=Ein stark genutzter Bereich enthält viele Nachrichtenkennungen. Sie bei jedem Lauf vollständig zu lesen kostet Zeit.
fido_auto_add=Fido-Bereiche automatisch anlegen
fido_auto_add-status=Bereich für eine noch unbekannte Kennung anlegen
fido_auto_add-help=Ohne diese Option wird eine Nachricht für eine unbekannte Kennung gezählt und verworfen.
fido_auto_add_conference=Zu Konferenz hinzufügen
fido_auto_add_conference-status=Konferenz für automatisch angelegte Bereiche
fido_auto_add_conference-help=
    # Zu Konferenz hinzufügen

    Konferenz, in der der Tosser selbstständig Bereiche anlegt.
    Neue Bereiche landen damit an einem bekannten statt an einem beliebigen Ort.
fido_pass_thru=PassThru aktivieren
fido_pass_thru-status=Bereiche ohne lokale Speicherung weiterreichen
fido_pass_thru-help=Ein Hub versorgt nachgelagerte Nodes auch mit Bereichen, die er selbst nicht liest. Jede Nachricht wird allen Gegenstellen angeboten, die die Kennung abonniert und die Nachricht noch nicht erhalten haben.
fido_make_response=Antwortnachrichten erzeugen
fido_make_response-status=AreaFix-Ergebnisse und Fehler per Netmail zurücksenden
fido_make_response-help=AreaFix-Befehle werden auch ausgeschaltet verarbeitet, aber es wird keine Ergebnis-Netmail erzeugt.
fido_area_fix_forwarding=AreaFix-Weiterleitung aktivieren
fido_area_fix_forwarding-status=Unbekannte AreaFix-Abonnements an einen übergeordneten Node weiterleiten
fido_area_fix_forwarding-help=Der erste andere konfigurierte Node erhält die weitergeleitete Anfrage. Den Uplink in der Node-Konfiguration vor den Downlinks eintragen.
fido_auto_add_passthru=Fido-Bereiche als PassThru anlegen
fido_auto_add_passthru-status=PassThru-Abonnement für unbekannte AreaFix-Kennungen erstellen
fido_auto_add_passthru-help=Erfordert aktiviertes PassThru. Die Kennung wird dem anfordernden Node hinzugefügt, ohne eine lokale Nachrichtenbasis anzulegen.
fido_re_address=Weitergeleitete Pakete umadressieren
fido_re_address-status=Weitergeleitete Pakete an den nächsten Node adressieren
fido_re_address-help=Ohne diese Option bleibt das ursprüngliche Endziel im Paketkopf. Das Bündel wird trotzdem für den nächsten Node eingereiht.
fido_route_echo_mail=Echomail weiterleiten
fido_route_echo_mail-status=Routing-Konfiguration für ausgehende Echomail-Pakete verwenden
fido_route_echo_mail-help=Gibt es für einen Ziel-Node eine Route, wird sein Echomail-Bündel im Ausgang des nächsten Nodes abgelegt.
fido_secure=Netmail absichern
fido_secure-status=Netmail von nicht konfigurierten Nodes getrennt halten
fido_secure-help=
    Netmail aus Paketen, deren FTN-Absenderadresse keinem konfigurierten
    Node entspricht, kommt in die gesicherte Netmail-Basis. Das entspricht
    den ~FIDO~-Node-Datensätzen von PCBoard: Geprüft wird der Absender-Node,
    nicht der Empfängername. Die Adresse ist nur eine Behauptung des Pakets;
    wichtigen Nodes daher in der Node-Konfiguration ein Paketpasswort zuweisen.
fido_sysop_change=SYSOP beim Import in FIDO_SYSOP ändern
fido_sysop_change-status=Generische SYSOP-Post soll keine Postbenachrichtigung des Board-Sysops auslösen
fido_sysop_change-help=
    # SYSOP in FIDO_SYSOP ändern

    Fido-Nachrichten sind oft allgemein an SYSOP adressiert. Umadressieren
    an FIDO_SYSOP verhindert, dass sie in Echomail-Bereichen das Kennzeichen
    für wartende Post des Board-Sysops setzen.
fido_directory_title=Fido-Verzeichniskonfiguration
fido_inbound=Eingehende Pakete
fido_inbound-status=Ablageort empfangener Daten
fido_inbound-help=
    # Eingehende Pakete

    Verzeichnis für empfangene Daten, bis der Tosser sie
    in die Nachrichtenbasen einliest.
fido_outbound=Ausgehende Pakete
fido_outbound-status=Hier wartet Post auf die nächste Verbindung
fido_outbound-help=
    # Ausgehende Pakete

    Verzeichnis für Post, die auf die nächste Verbindung zu einem Node
    wartet. Alles hier Befindliche ist noch nicht zugestellt.
fido_netmail=Netmail-Basis
fido_netmail-status=Nachrichtenbasis für eintreffende Netmail
fido_netmail-help=
    # Netmail-Basis

    Nachrichtenbasis für eintreffende Netmail, also private Post
    zwischen Systemen statt der Echomail-Konferenzen.
fido_bad_netmail=Gesicherte Netmail-Basis
fido_bad_netmail-status=Ablage für Netmail von nicht konfigurierten Nodes
fido_bad_netmail-help=Wird nur bei aktivierter Netmail-Absicherung verwendet.
fido_bad_packets=Verzeichnis für fehlerhafte Pakete
fido_bad_packets-status=Ablage für Pakete, die nicht gelesen werden konnten
fido_bad_packets-help=
    # Verzeichnis für fehlerhafte Pakete

    Unlesbare Pakete werden hierher verschoben und im Bericht genannt.
    Sonst blieben sie im Eingang und würden bei jedem Lauf erneut
    versucht und beanstandet. Häufige Gründe sind falsche Paketpasswörter
    oder beschädigte Dateien.
fido_new_areas=Neue Bereiche
fido_new_areas-status=Speicherort der Basis eines neu angelegten Bereichs
fido_new_areas-help=Wird nur bei automatischer Anlage von Fido-Bereichen verwendet.
fido_nodelist=Nodeliste
fido_nodelist-status=Liste zum Nachschlagen von Systemen (LEER=keine)
fido_nodelist-help=
    # Nodeliste

    Jedes Fidonet-Netz veröffentlicht eine Liste seiner Systeme. Nodes
    ohne eigenen konfigurierten Host werden hier nachgeschlagen, damit
    Host und Port nicht von Hand übernommen werden müssen.

    Die Liste selbst angeben, keinen kompilierten Index; sie wird direkt
    gelesen. Ohne Liste muss jeder Node seinen eigenen Host angeben.
fido_address_title=Fidonet-Adresskonfiguration
fido_address_editor=Adresse
fido_address_header_node=Node
fido_address_header_primary=Standard
fido_address_header_domain=Domain
fido_address_node=Node
fido_address_node-status=Adresse dieses Boards im Format zone:net/node[.point]
fido_address_invalid=Vollständige Adresse mit Zone, Netz und Node ungleich null eingeben
fido_address_domain=Domain
fido_address_domain-status=Netz der Adresse; wird in binkp nach einem '@' gesendet
fido_node_title=Fidonet-Node-Konfiguration
fido_node_editor=Node
fido_node_header_node=Node
fido_node_header_host=Host
fido_node_header_areas=Bereiche
fido_node_node=Node
fido_node_node-status=Adresse des Systems für den Nachrichtenaustausch
fido_node_domain=Domain
fido_node_domain-status=Netz, zu dem dieser Node gehört
fido_node_host=Host
fido_node_host-status=Host für die Verbindung zu diesem Node
fido_node_port=Port
fido_node_port-status=binkp-Port; 24554, sofern der Node nichts anderes angibt
fido_node_password=Sitzungspasswort
fido_node_password-status=Vom Node bei einer Verbindung erwartetes Passwort
fido_node_packet_password=Paketpasswort
fido_node_packet_password-status=Acht Zeichen, die ein Paket dieses Nodes enthalten muss
fido_node_packet_password-help=
    # Paketpasswort

    Acht Zeichen, die Pakete dieses Nodes enthalten müssen und mit denen
    Pakete an ihn versehen werden. Ein Paket mit seiner Adresse, aber
    ohne dieses Passwort bleibt im Eingangsverzeichnis. Leer lassen,
    um Pakete ungeprüft anzunehmen, wie PCBoard bei einem
    ~FIDO~-Datensatz ohne Passwort.
fido_node_areafix_password=AreaFix-Passwort
fido_node_areafix_password-status=Erwartetes Passwort im Betreff von AreaFix-Anfragen dieses Nodes
fido_node_areafix_password-help=Leer lassen, um einen leeren AreaFix-Betreff zu akzeptieren. Dieses Passwort ist unabhängig von binkp-Sitzungs- und Paketpasswörtern.
fido_node_areas=Bereiche
fido_node_areas-status=Durch Leerzeichen getrennte Echomail-Kennungen dieses Nodes
fido_node_areas-help=
    # Bereiche

    Echomail-Kennungen dieses Nodes. Lokal geschriebene Post wird einem
    Node nur für seine abonnierten Bereiche angeboten, dabei aber jedem
    Abonnenten. Leer bedeutet: Der Node erhält keine Echomail.
fido_origin=Origin
fido_origin-status=An Echomail angehängte Zeile für Bereiche ohne eigene Origin
fido_origin-help=
    # Origin

    Zeile, die jeder auf diesem Board geschriebenen Echomail angehängt wird.
    In Fidonet sind Boardname und Erreichbarkeit üblich. Manche Netze
    verlangen eine Origin; deshalb vor dem ersten Export festlegen.

    PCBoard führte hier Origins nach Konferenzbereichen auf. Stattdessen
    besitzt jetzt jeder Bereich im Bereichseditor eine eigene Fido-Origin.
    Diese Zeile wird von allen Bereichen ohne eigene Angabe verwendet.
fido_route_title=Fidonet-Routing-Konfiguration
fido_route_editor=Route
fido_route_header_destination=Ziel
fido_route_header_via=Über
fido_route_destination=Ziel
fido_route_destination-status=Endgültige FTN-Adresse, für die diese Route gilt
fido_route_via=Über Node
fido_route_via-status=Konfigurierter Node, der das Paket als nächste Station erhält
fido_route_via-help=Die Adresse muss auch in der Node-Konfiguration vorhanden sein.

qwk_settings_title=QWK-Einstellungen
qwk_bbs_label=BBS-Informationen
qwk_bbs_name=Name
qwk_bbs_name-status=BBS-Name
qwk_bbs_name-help=
    # Name

    Boardname in heruntergeladenen QWK-Paketen. Unter diesem Namen
    zeigt der Offline-Reader das Board an.
qwk_bbs_city_and_state=Stadt und Region
qwk_bbs_city_and_state-status=Stadt und Region des BBS
qwk_bbs_city_and_state-help=
    # Stadt und Region

    Standort des Boards, wie er im QWK-Paket steht.
qwk_bbs_phone_number=Telefon
qwk_bbs_phone_number-status=BBS-Telefonnummer
qwk_bbs_phone_number-help=
    # Telefon

    Telefonnummer im QWK-Paket. Bei einem nur über Netzwerk
    erreichbaren Board leer lassen.
qwk_bbs_sysop_name=Sysop
qwk_bbs_sysop_name-status=BBS-Sysop
qwk_bbs_sysop_name-help=
    # Sysop

    Sysop-Name im QWK-Paket.
qwk_bbs_id=ID
qwk_bbs_id-status=BBS-Kennung
qwk_bbs_id-help=
    # ID

    Kurze Boardkennung mit höchstens acht Zeichen. Sie wird zum
    Paketnamen und sollte unter den vom Benutzer gelesenen Boards eindeutig sein.
qwk_files_label=QWK-Dateien
qwk_welcome_screen=Begrüßungsbildschirm
qwk_welcome_screen-status=QWK-Begrüßungsbildschirm
qwk_welcome_screen-help=
    # Begrüßungsbildschirm

    Wird als Begrüßung in das QWK-Paket aufgenommen und vom
    Offline-Reader des Benutzers angezeigt.
qwk_goodbye_screen=Abschiedsbildschirm
qwk_goodbye_screen-status=QWK-Abschiedsbildschirm
qwk_goodbye_screen-help=
    # Abschiedsbildschirm

    Wird als Abschied in das QWK-Paket aufgenommen.
qwk_news_sceen=Neuigkeitenbildschirm
qwk_news_sceen-status=QWK-Neuigkeitenbildschirm
qwk_news_sceen-help=
    # Neuigkeitenbildschirm

    Neuigkeiten im QWK-Paket. Damit bleiben auch rein offline
    lesende Benutzer über das Geschehen auf dem Laufenden.
message_box_info_title= Information
message_box_warning_title= Warnung
message_box_error_title= Fehler
message_box_dismiss= ENTER drücken
no_file_name_given=Für diesen Eintrag ist kein Dateiname konfiguriert.