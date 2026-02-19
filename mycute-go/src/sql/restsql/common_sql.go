package restsql

type Type uint8

const (
	READ Type = iota + 1
	SHARE
	USE
	DELETE
	UPDATE
)

func IsRead(t *Type) bool {
	if t == nil {
		return false
	}
	if *t == READ {
		return true
	}
	return false
}

func IsShare(t *Type) bool {
	if t == nil {
		return false
	}
	if *t == SHARE {
		return true
	}
	return false
}

func IsUse(t *Type) bool {
	if t == nil {
		return false
	}
	if *t == USE {
		return true
	}
	return false
}

func IsDelete(t *Type) bool {
	if t == nil {
		return false
	}
	if *t == DELETE {
		return true
	}
	return false
}

func IsUpdate(t *Type) bool {
	if t == nil {
		return false
	}
	if *t == UPDATE {
		return true
	}
	return false
}
