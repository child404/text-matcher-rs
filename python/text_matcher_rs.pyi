from typing import List, Optional


class Candidate:
    def __init__(
        self, text: str, candidate_text: str, sensitivity: float, file_found: str
    ): ...
    @property
    def text(self) -> str: ...
    @property
    def similarity(self) -> float: ...
    @property
    def file_found(self) -> str: ...


class TextMatcher:
    def __init__(self, sensitivity: float, keep: int, path_to_candidates: str): ...
    @property
    def sensitivity(self) -> float: ...
    @sensitivity.setter
    def sensitivity(self, value: float): ...
    @property
    def keep(self) -> int: ...
    @keep.setter
    def keep(self, value: int): ...
    @property
    def candidates(self) -> Optional[List[str]]: ...
    def set_candidates(self, new_candidates_file: str): ...
    def find_matches(self, text: str) -> List[Candidate]: ...


def find_matches_in_dir(
    sens: float, keep: int, text: str, path_to_dir: str, num_of_threads: Optional[int]
) -> Optional[List[Candidate]]: ...