WITH UserVoteStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(V.Id) AS TotalVotes,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        SUM(CASE WHEN V.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes
    FROM Users U
    LEFT JOIN Votes V ON U.Id = V.UserId
    GROUP BY U.Id, U.DisplayName
),
PostStats AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        COALESCE(UPV.UpVoteCount, 0) AS UpVoteCount,
        COALESCE(DOV.DownVoteCount, 0) AS DownVoteCount,
        P.OwnerUserId,
        U.DisplayName AS OwnerDisplayName,
        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount
    FROM Posts P
    LEFT JOIN (
        SELECT PostId, COUNT(*) AS UpVoteCount
        FROM Votes 
        WHERE VoteTypeId = 2
        GROUP BY PostId
    ) UPV ON P.Id = UPV.PostId
    LEFT JOIN (
        SELECT PostId, COUNT(*) AS DownVoteCount
        FROM Votes 
        WHERE VoteTypeId = 3
        GROUP BY PostId
    ) DOV ON P.Id = DOV.PostId
    JOIN Users U ON P.OwnerUserId = U.Id
),
RecentPostHistory AS (
    SELECT 
        PH.PostId,
        PHT.Name AS ChangeType,
        PH.CreationDate AS ChangeDate,
        ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC, PH.Id) AS rn
    FROM PostHistory PH
    JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
    WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
)

SELECT 
    PS.PostId,
    PS.Title,
    PS.CreationDate,
    PS.Score,
    PS.ViewCount,
    PS.UpVoteCount,
    PS.DownVoteCount,
    PS.CommentCount,
    UVS.TotalVotes AS UserTotalVotes,
    UVS.UpVotes AS UserUpVotes,
    UVS.DownVotes AS UserDownVotes,
    RP.ChangeType,
    RP.ChangeDate
FROM PostStats PS
LEFT JOIN UserVoteStats UVS ON PS.OwnerUserId = UVS.UserId
LEFT JOIN RecentPostHistory RP ON PS.PostId = RP.PostId AND RP.rn = 1
WHERE PS.Score > 0
  AND PS.ViewCount > 100
  AND (PS.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' 
       OR PS.OwnerUserId IS NULL)
ORDER BY PS.Score DESC, PS.ViewCount DESC;