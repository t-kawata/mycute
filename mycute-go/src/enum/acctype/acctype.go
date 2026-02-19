package acctype

type AccType uint8

const (
	CORP AccType = 1
	INDI AccType = 2
)

func (t AccType) Val() uint8 {
	return uint8(t)
}
