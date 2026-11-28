"""Select the native build when the batch includes I09."""
import os

from client_replay import selected_rows


if __name__ == "__main__":
    try:
        selected, _ = selected_rows(os.environ.get("OTERYN_E2E_ROW_BATCH"))
    except ValueError:
        # The normal row validator reports invalid batch selection.
        selected = ()
    print(int("I09.client" in selected))
