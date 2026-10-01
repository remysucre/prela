WITH UserStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        U.Reputation,
        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
        COUNT(DISTINCT P.Id) AS TotalPosts,
        COUNT(DISTINCT C.Id) AS TotalComments
    FROM 
        Users U
    LEFT JOIN 
        Votes V ON U.Id = V.UserId
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    LEFT JOIN 
        Comments C ON P.Id = C.PostId
    GROUP BY 
        U.Id, U.DisplayName, U.Reputation
),
PostHistorySummary AS (
    SELECT 
        PH.PostId,
        MAX(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN PH.CreationDate ELSE NULL END) AS LastClosedDate,
        COUNT(CASE WHEN PH.PostHistoryTypeId = 9 THEN 1 ELSE NULL END) AS TotalRollbackTags
    FROM 
        PostHistory PH
    GROUP BY 
        PH.PostId
),
ActivePosts AS (
    SELECT 
        P.Id, 
        P.Title,
        P.OwnerUserId,
        P.ViewCount,
        PH.LastClosedDate,
        PH.TotalRollbackTags,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
    FROM 
        Posts P
    LEFT JOIN 
        PostHistorySummary PH ON P.Id = PH.PostId
    WHERE 
        P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
        AND (PH.LastClosedDate IS NULL OR PH.LastClosedDate > P.LastActivityDate)
)
SELECT 
    U.UserId,
    U.DisplayName AS UserDisplayName,
    U.Reputation AS UserReputation,
    COUNT(DISTINCT A.Id) AS ActivePostCount,
    SUM(A.ViewCount) AS TotalViewCount,
    AVG(U.TotalUpVotes - U.TotalDownVotes) AS AverageVoteDifference,
    STRING_AGG(DISTINCT A.Title, ', ' ORDER BY A.Title) AS ActivePostTitles
FROM 
    UserStats U
JOIN 
    ActivePosts A ON U.UserId = A.OwnerUserId
WHERE 
    U.TotalPosts > 0
    AND U.Reputation > 100
GROUP BY 
    U.UserId, U.DisplayName, U.Reputation
HAVING 
    COUNT(DISTINCT A.Id) > 5
ORDER BY 
    TotalViewCount DESC, UserReputation DESC
LIMIT 10;