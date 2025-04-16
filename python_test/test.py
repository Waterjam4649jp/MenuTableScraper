import menu_scraper 

tables = menu_scraper.get_menu_tables()
for i, table in enumerate(tables):
    print(f"[TABLE {i}]")
    for row in table:
        print(row)

