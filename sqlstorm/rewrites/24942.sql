WITH RankedUsers AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        U.Reputation,
        RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
    FROM Users U
    WHERE U.Reputation > 500
),
RecentPosts AS (
    SELECT 
        P.Id AS PostId,
        P.OwnerUserId,
        P.CreationDate,
        P.Score,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC, P.Id) AS PostRank
    FROM Posts P
    WHERE P.CreationDate > (cast('2024-10-01' as date) - INTERVAL '30 days')
),
ClosedPosts AS (
    SELECT 
        PH.PostId,
        P.OwnerUserId,
        PH.CreationDate AS CloseCreationDate,
        COUNT(*) AS CloseCount,
        STRING_AGG(DISTINCT C.UserDisplayName, ', ') AS ClosingUserNames
    FROM PostHistory PH
    JOIN Posts P ON P.Id = PH.PostId
    LEFT JOIN Comments C ON C.PostId = PH.PostId
    WHERE PH.PostHistoryTypeId IN (10, 11) 
    GROUP BY PH.PostId, P.OwnerUserId, PH.CreationDate
),
VotingStats AS (
    SELECT 
        V.PostId,
        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
        COUNT(CASE WHEN V.VoteTypeId = 10 THEN 1 END) AS DeleteVotes
    FROM Votes V
    GROUP BY V.PostId
),
FinalReport AS (
    SELECT 
        RU.UserId,
        RU.DisplayName,
        RU.Reputation,
        COALESCE(RP.PostId, 0) AS RecentPostId,
        COALESCE(RP.Score, 0) AS RecentPostScore,
        CP.CloseCount,
        CP.ClosingUserNames,
        VS.UpVotes,
        VS.DownVotes,
        (CASE 
            WHEN CP.CloseCount > 0 THEN 'Closed Posts'
            ELSE 'Active Posts' 
        END) AS PostStatus
    FROM RankedUsers RU
    LEFT JOIN RecentPosts RP ON RU.UserId = RP.OwnerUserId AND RP.PostRank = 1
    LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId
    LEFT JOIN VotingStats VS ON COALESCE(RP.PostId, 0) = VS.PostId
    WHERE RU.ReputationRank <= 10
)
SELECT 
    UserId,
    DisplayName,
    Reputation,
    RecentPostId,
    RecentPostScore,
    CloseCount,
    ClosingUserNames,
    UpVotes,
    DownVotes,
    PostStatus
FROM FinalReport
WHERE UserId IS NOT NULL
ORDER BY Reputation DESC, UpVotes DESC NULLS LAST
LIMIT 100;